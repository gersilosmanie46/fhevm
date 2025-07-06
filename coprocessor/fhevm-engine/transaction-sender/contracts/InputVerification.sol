// SPDX-License-Identifier: BSD-3-Clause-Clear
pragma solidity ^0.8.28;

contract InputVerification {
    event VerifyProofResponse(uint256 indexed zkProofId, bytes32[] ctHandles, bytes[] signatures);
    event RejectProofResponse(uint256 indexed zkProofId);

    error CoprocessorAlreadyVerified(uint256 zkProofId, address txSender, address signer);
    error CoprocessorAlreadyRejected(uint256 zkProofId, address txSender, address signer);

    bool public alreadyVerifiedRevert;
    bool public alreadyRejectedRevert;
    bool public otherRevert;

    constructor(bool _alreadyVerifiedRevert, bool _alreadyRejectedRevert, bool _otherRevert) {
        alreadyVerifiedRevert = _alreadyVerifiedRevert;
        alreadyRejectedRevert = _alreadyRejectedRevert;
        otherRevert = _otherRevert;
    }

    function verifyProofResponse(
        uint256 zkProofId,
        bytes32[] calldata handles,
        bytes calldata signature
    ) external {
        if (otherRevert) revert("Other revert");
        if (alreadyVerifiedRevert) revert CoprocessorAlreadyVerified(zkProofId, msg.sender, msg.sender);

        emit VerifyProofResponse(zkProofId, handles, toBytesArray(signature));
    }

    function rejectProofResponse(uint256 zkProofId) external {
        if (otherRevert) revert("Other revert");
        if (alreadyRejectedRevert) revert CoprocessorAlreadyRejected(zkProofId, msg.sender, msg.sender);

        emit RejectProofResponse(zkProofId);
    }

    function toBytesArray(bytes calldata signature) private pure returns (bytes[] memory signatures) {
        signatures = new bytes[](1);
        signatures[0] = signature;
    }
}
