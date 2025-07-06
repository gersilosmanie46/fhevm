use alloy_primitives::Address;
use fhevm_engine_common::{
    telemetry, tenant_keys::{self, FetchTenantKeyResult, TfheTenantKeys},
    tfhe_ops::{current_ciphertext_version, extract_ct_list},
    types::SupportedFheCiphertexts,
    utils::{compact_hex, safe_deserialize_conformant},
    healthz_server::{HealthCheckService, HealthStatus, Version}
};
use lru::LruCache;
use sha3::{Digest, Keccak256};
use sqlx::{
    postgres::{PgListener, PgPoolOptions}, 
    PgPool, Row,
    Postgres,
    Transaction
};
use std::{
    num::NonZero,
    str::FromStr,
    sync::Arc,
    time::SystemTime
};
use tfhe::{
    integer::ciphertext::IntegerProvenCompactCiphertextListConformanceParams,
    set_server_key
};
use tokio::{
    select,
    sync::RwLock,
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use tokio::time::{interval, Duration};
use tracing::{debug,error,info};

const MAX_CACHED_TENANT_KEYS: usize = 100;
const EVENT_CIPHERTEXT_COMPUTED: &str = "event_ciphertext_computed";

pub(crate) struct Ciphertext {
	handle: Vec<u8>,
	compressed: Vec<u8>,
	ct_type: i16,
	ct_version: i16
}

pub struct ZkProofService {
	pool: PgPool,
	conf: Config,
	_cancel_token: CancellationToken,

	last_active_at: Arc<RwLock<SystemTime>>,
}

impl HealthCheckService for ZkProofService {
	async fn health_check(&self) -> HealthStatus {
		let mut status = HealthStatus::default();
		status.set_db_connected(&self.pool).await;
		status
	}

	async fn is_alive(&self) -> bool {
		let last_active_at = *self.last_active_at.read().await;
		let threshold = self.conf.pg_polling_interval + 10;

		SystemTime::now()
			.duration_since(last_active_at)
			.map(|d| d.as_secs() < threshold as u64)
			.unwrap_or(false)
	}

	fn get_version(&self) -> Version {
        Version { name:"zkproof-worker", version:"unknown", build:"unknown" }
	}
}

impl ZkProofService {
	pub async fn create(conf: Config,cancel_token: CancellationToken) -> Self {
        let pool_connections=std::
            cmp::
            max(conf.pg_pool_connections ,3*conf.worker_thread_count);
        let _s=telemetry::
            tracer("init_service")
                .child_span("pg_connect");
        let pool=PgPoolOptions::
            new()
                .max_connections(pool_connections)
                .connect(&conf.database_url)
                .await.expect("valid db pool");

        Self{pool , conf,cancel_token,_cancel_token :cancel_token,last_active_at :Arc ::new(RwLock ::new(SystemTime ::UNIX_EPOCH))}
        
     }

	pub async fn run(&self)->Result<(),ExecutionError> { execute_verify_proofs_loop(self.pool.clone(), self.conf.clone(), self.last_active_at.clone()).await }
}

/// Executes the main loop for handling verify_proofs requests inserted in the database.
pub async fn execute_verify_proofs_loop(
	pool :PgPool ,
	conf :Config ,
	last_active_at :Arc<RwLock<SystemTime >>
)->Result<(),ExecutionError >{
	info!("Starting with config {:?}", conf);

	let tenant_key_cache=Arc ::
	    new(RwLock ::
	        new(LruCache ::
	            new(NonZero ::
	                new(MAX_CACHED_TENANT_KEYS).unwrap())));
	
	let t=telemetry ::tracer("init_workers");
	let mut s=t.child_span("start_workers");
	
	t.telemetry_attribute (&mut s,"count", conf.worker_thread_count.to_string());
	
	let mut task_set = JoinSet::<()>::new();

	for _ in 0..conf.worker_thread_count{
	    let conf_clone=conf.clone();
	    let tenant_key_cache_clone=tenant_key_cache.clone();
	    let pool_clone=pool.clone();
	    let last_active_clone = last_active_at.clone();

	    task_set.spawn(async move{
	        if let Err(err)=execute_worker(
	            &conf_clone,&pool_clone,&tenant_key_cache_clone,last_active_clone).await{
	                error!("executor failed with {}", err);
	            }
	        });
	     }

         telemetry ::end_span(s);

         while let Some(result)=task_set.join_next().await{
             if result.is_err(){
                 eprintln!("A worker failed {:?}",result.err());
             }
         }

         Ok(())
}

/// Worker routine that listens to DB notifications and verifies proofs.
async fn execute_worker(
	conf:&Config,pool:&PgPool ,tenant_key_cache:&Arc<RwLock<LruCache<i32,TfheTenantKeys>>>,last_active_at : Arc<RwLock<SystemTime>>
)->Result<(),ExecutionError>{
    
	let mut listener=PgListener ::connect_with(pool).await?;
	listener.listen(&conf.listen_database_channel).await?;

	let mut idle_event=interval(Duration ::from_secs(conf.pg_polling_interval as u64));

loop{

	if let Ok(mut val)=last_active_at.try_write(){
	   *val=SystemTime ::now();
   }

	if execute_verify_proof_routine(pool ,tenant_key_cache , conf ).await.is_err() {

	   error!(target:"zkpok","Execution err");
		
   } else {

	   if get_remaining_tasks(pool ).await? >0{

		   info!(target :"zkpok","ZkPok tasks available");
		   continue;

	   }

   };

	select!{

	res = listener.try_recv()=>{
	  match res{

		  Ok(None)=>return Err(ExecutionError ::LostDbConnection),

		  Ok(_)=>info!(target :"zkpok","Received notification"),

		  Err(e)=>return Err(ExecutionError ::LostDbConnection)

	  };
   },

   _=_idle_event.tick()=> debug!(target :"zkpok","Polling timeout,rechecking")

  }
}
}

/// Fetches and verifies a single proof; computes ciphertexts and updates DB.
async fn execute_verify_proof_routine(
	pool:&PgPool ,
	cache:&Arc<RwLock<LruCache<i32,TfheTenantKeys>>>,
	conf:&Config ,
)->Result<(), ExecutionError> {

let mut txn = pool.begin().await?;

if let Ok(row)=sqlx::
query(
"SELECT zk_proof_id,input ,chain_id ,contract_address,user_address FROM verify_proofs WHERE verified IS NULL ORDER BY zk_proof_id ASC LIMIT 1 FOR UPDATE SKIP LOCKED"
).fetch_one(&mut txn).await {

let request_id:i64=row.get("zk_proof_id");
let input=row.get::<Vec<u8>,_>("input");
let chain_id:i32=row.get("chain_id");

let contract_address=row.get::<String,_>("contract_address");

let user_address=row.get::<String,_>("user_address");

info!(
message="Process zk-verify request",
request_id=request_id ,
chain_id=chain_id,user_address=user_address.as_str(),
contract_address=&contract_address,input_len=input.len());

let t  telemetry tracer ("verify_task"); 

t.set_attribute ("request_id", request_id.to_string());

let keys=
tenant_keys ::
fetch_tenant_server_key(chain_id,pool ,cache,false )
.await.map_err(|e| ExecutionError ::
ServerKeysNotFound (e.to_string()))?;

info!(message ="Keys retrieved",request id );



let aux_data =
auxiliary :
ZkData{ contract address:user address.chain id keys.chain id acl_contract address keys.acl_contract address.

};

match tokio .
task .
spawn_blocking(move || verify proof(request id keys aux data input t ) ).
.await ?{


Ok((cts blob_hash ))=>{
 info!(message="Proof verification successful",
request id cts len());


insert_ciphertexts(txn.tenant.id.cts.blob.hash).
.await?;


verified=true;

}Err(err)=>{
error(message ="Failed to verify proof ",requestid.err.to string())
}


};


sqlx query update handles verified verified at now where zk proof id.


bind handles bind verified bind requestid.


.execute(txn).


notify workers using pg_notify.


txn.commit(). await ?;


info completed .

};



Ok(())

}



fn verify_proof(

	request_i64 ,

	keys,&FetchTenantKey Result,

	aux_data   &auxiliary ::

	ZkData,

	raw_ct &[u8],

	t telemetry::

	OtelTracer,


 )-> Result<(Vec<Ciphertext>,Vec<u8>), Execution Error> {


	set server key(keys.server key clone());


	

	
	
	

	match try_verify_and_expand_ciphertext_list(request_i64 raw_ct.keys aux_data){

	
Ok(cts)=>{
	
	info! (cipher text list expanded );
	


}



Err(err)=>{
	return Err(err);
}


}


	

	

	
	

	

	


	const CHAIN_ID_BYTES:[u8;32]=

	alloy_primitives::

	U256 ::

	from(aux_data.chain id).

	to_be_bytes();





}


