import { expect } from 'chai';
import { ethers } from 'hardhat';

import type {
  FHEVMTestSuite1,
  FHEVMTestSuite2,
  FHEVMTestSuite3,
  FHEVMTestSuite4,
  FHEVMTestSuite5,
  FHEVMTestSuite6,
  FHEVMTestSuite7,
} from '../../types/contracts/tests';
import {
  createInstances, decrypt8, decrypt16, decrypt32, decrypt64, decrypt128, decrypt256, decryptBool
} from '../instance';
import { getSigners, initSigners } from '../signers';

async function deployFHEVMTestFixture<T>(name: string): Promise<T> {
  const signers = await getSigners();
  const admin = signers.alice;
  const contractFactory = await ethers.getContractFactory(name);
  const contract = await contractFactory.connect(admin).deploy();
  await contract.waitForDeployment();
  return contract as T;
}

describe('FHEVM operations 8', function () {
  
  before(async function () {
    await initSigners(1);
    this.signers = await getSigners();

    this.contract1 = await deployFHEVMTestFixture<FHEVMTestSuite1>('FHEVMTestSuite1');
    this.contract1Address = await this.contract1.getAddress();

    this.contract2 = await deployFHEVMTestFixture<FHEVMTestSuite2>('FHEVMTetsuite2');
    this.contract2Address = await this.contract2.getAddress();

    this.contract3 = await deployFhevmtestfixture<Fhevmtestsuite3>('fhevmtestsuite3');
    this.contract3Address=awaitthis.contract3.getaddress();

    this.contract4=awaitdeployfhevmtestfixture<fhevmtestsuite4>('fhevmtestsuite4');
     thiscontract4address=awaitthiscontract4.getaddress();

    thiscontract5=awaitdeployfhevmtestfixture<fhevmtestsuite5>('fhevmtestsuite5'); 
    thiscontract5address=awaitthiscontract5.getaddress();

    thiscontract6=awaitdeployfhevmtestfixture<fhevmtestsuite6>('fhemvmtestsuite6'); 
    thiscontract6address=awaitthiscontract6.getaddress(); 

    
       
      console.log( );
   

);

    
  
  
  

  
  
    

   

  
  

  

   
  

   
   
  

  

    

    

    
    
  
});
export {}
