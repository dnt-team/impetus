import { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

const config: HardhatUserConfig = {
  solidity: "0.8.17",
  networks: {
    local: {
      url: "http://localhost:9933",
      accounts: ['0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342'],
      chainId: 42
    }
  },
  gasReporter: {
    currency: 'USD',
    coinmarketcap: 'd418ace3-6c4c-4608-97ff-2162c3492418'
  }
};

export default config;
