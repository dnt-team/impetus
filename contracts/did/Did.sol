// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The Batch contract's address.
address constant DID_ADDRESS = 0x0000000000000000000000000000000000000806;

/// @dev The Batch contract's instance.
Did constant DID_CONTRACT = Did(DID_ADDRESS);

/// @author The Impetus Team
/// @title Pallet DiD Interface
/// @dev The interface through which solidity contracts will interact with DiD Pallet
/// We follow this same interface including four-byte function selectors, in the precompile that
/// wraps the pallet
/// @custom:address 0x0000000000000000000000000000000000000806
interface Did {
    function externalIdAddress(
        address user,
        string calldata provider
    ) external view returns (string memory);
}
