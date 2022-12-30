// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @author The Impetus Team
/// @title Pallet Betting Interface
/// @dev The interface through which solidity contracts will interact with Betting
/// We follow this same interface including four-byte function selectors, in the precompile that
/// wraps the pallet
/// @custom:address 0x0000000000000000000000000000000000000803
interface Betting {
    function bet(string calldata round_id, uint128 bet_id, uint256 amount) external;
}
