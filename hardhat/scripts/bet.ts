import { ethers } from "hardhat";
// @ts-ignore
// Get this file from build
import { abi } from "../artifacts/contracts/BettingInterface.sol/Betting.json";
async function main() {
	const address = "0x0000000000000000000000000000000000000803";
    console.log(1)
    const signer = await ethers.getSigner("0x6be02d1d3665660d22ff9624b7be0551ee1ac91b")
    console.log(1)

	const erc20 = new ethers.Contract(address, abi, signer);
	try {
	await erc20.bet("0x023f6da8c2c62634ddf1a786037938fe63222d534942c80e957beb965c66f2ec", 1, 100)
		
	} catch (error) {
		console.log(error)
	}
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
	console.error(error);
	process.exitCode = 1;
});
