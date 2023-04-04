// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @author The Impetus Team
/// @title Pallet Lucky Number Interface
/// @dev The interface through which solidity contracts will interact with LuckyNumber Pallet
/// We follow this same interface including four-byte function selectors, in the precompile that
/// wraps the pallet
/// @custom:address 0x0000000000000000000000000000000000000804
interface LuckyNumber {
    function buyTicket(uint8 number, uint256 amount) external;
}