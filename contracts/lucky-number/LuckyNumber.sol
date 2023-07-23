// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;


/// @dev The Batch contract's address.
address constant LUCKY_NUMBER_ADDRESS = 0x0000000000000000000000000000000000000808;

/// @dev The Batch contract's instance.
LuckyNumber constant LUCKY_NUMBER_CONTRACT = LuckyNumber(LUCKY_NUMBER_ADDRESS);


/// @author The Impetus Team
/// @title Pallet Lucky Number Interface
/// @dev The interface through which solidity contracts will interact with LuckyNumber Pallet
/// We follow this same interface including four-byte function selectors, in the precompile that
/// wraps the pallet
/// @custom:address 0x0000000000000000000000000000000000000804
interface LuckyNumber {
    function buyTickets(uint8[] calldata numbers, uint256[] calldata amounts) external;
}