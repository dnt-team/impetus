// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The Batch contract's address.
address constant GIVEAWAY_ADDRESS = 0x0000000000000000000000000000000000000805;

/// @dev The Batch contract's instance.
Giveaway constant GIVEAWAY_CONTRACT = Giveaway(GIVEAWAY_ADDRESS);

/// @author The Impetus Team
/// @title Pallet Giveaway Interface
/// @dev The interface through which solidity contracts will interact with Giveaway Pallet
/// We follow this same interface including four-byte function selectors, in the precompile that
/// wraps the pallet
/// @custom:address 0x0000000000000000000000000000000000000805
interface Giveaway {
    function createGiveaway(
        string calldata name,
        uint32 startBlock,
        uint32 endBlock,
        uint8 kycStatus,
        uint8 randomType,
        uint8 assetType,
        uint32 assetId,
        uint256 amount,
        uint32 maxJoin
    ) external;

    function participate(uint32 index) external;
    function claimReward(uint32 index) external;
}
