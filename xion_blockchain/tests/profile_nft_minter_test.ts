import { CosmWasmClient, SigningCosmWasmClient, Secp256k1HdWallet } from "cosmwasm";
import { assert, expect } from "chai";

describe("ProfileNFTMinter Contract Tests", () => {
  const contractAddress = process.env.PROFILE_NFT_MINTER_ADDRESS || "";
  const rpcEndpoint = process.env.RPC_ENDPOINT || "http://localhost:26657";
  
  let client: CosmWasmClient;
  let signingClient: SigningCosmWasmClient;
  let wallet: Secp256k1HdWallet;
  let userAddress: string;
  
  before(async () => {
    client = await CosmWasmClient.connect(rpcEndpoint);
    
    // Set up wallet for testing
    const mnemonic = "your test mnemonic here";
    wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic);
    const [firstAccount] = await wallet.getAccounts();
    userAddress = firstAccount.address;
    
    signingClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      wallet
    );
  });
  
  describe("Instantiation", () => {
    it("should successfully instantiate the contract", async () => {
      // This is a read-only test, assuming contract is already deployed
      const contractInfo = await client.getContract(contractAddress);
      expect(contractInfo).to.exist;
      expect(contractInfo.address).to.equal(contractAddress);
    });
  });
  
  describe("NFT Minting", () => {
    it("should mint a profile NFT", async () => {
      const metadata = {
        name: "Test Profile",
        description: "This is a test profile NFT",
        image: "ipfs://Qm123456789abcdef",
        attributes: [
          {
            trait_type: "Background",
            value: "Blue"
          },
          {
            trait_type: "Avatar Style",
            value: "Pixel"
          }
        ]
      };
      
      const metadataUri = JSON.stringify(metadata);
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          mint_profile_nft: {
            metadata_uri: metadataUri
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Extract token_id from logs
      const tokenIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "token_id");
      
      expect(tokenIdLog).to.exist;
      const tokenId = tokenIdLog?.value;
      expect(tokenId).to.be.a("string");
    });
    
    it("should mint an authorized profile NFT", async () => {
      const metadata = {
        name: "Authorized Profile",
        description: "This is an authorized profile NFT",
        image: "ipfs://Qm987654321fedcba",
        attributes: [
          {
            trait_type: "Background",
            value: "Red"
          },
          {
            trait_type: "Avatar Style",
            value: "Realistic"
          }
        ]
      };
      
      const metadataUri = JSON.stringify(metadata);
      const recipient = userAddress; // Self-mint for test purposes
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          mint_authorized_profile: {
            recipient: recipient,
            metadata_uri: metadataUri
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("NFT Updates", () => {
    let tokenId: string;
    
    before(async () => {
      // Create an NFT to update
      const metadata = {
        name: "Update Test Profile",
        description: "This is a profile to test updates",
        image: "ipfs://QmTestUpdate123",
        attributes: []
      };
      
      const metadataUri = JSON.stringify(metadata);
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          mint_profile_nft: {
            metadata_uri: metadataUri
          }
        },
        "auto"
      );
      
      const tokenIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "token_id");
      
      tokenId = tokenIdLog?.value || "1";
    });
    
    it("should update a profile NFT's metadata", async () => {
      const newMetadata = {
        name: "Updated Profile",
        description: "This profile has been updated",
        image: "ipfs://QmUpdated456",
        attributes: [
          {
            trait_type: "Status",
            value: "Updated"
          }
        ]
      };
      
      const newMetadataUri = JSON.stringify(newMetadata);
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          update_profile_metadata: {
            token_id: tokenId,
            metadata_uri: newMetadataUri
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("Query Functions", () => {
    it("should get profile NFT owner", async () => {
      // Using a token created in previous tests
      const tokenId = "1"; // Assuming this token exists
      
      const result = await client.queryContractSmart(contractAddress, {
        owner_of: {
          token_id: tokenId
        }
      });
      
      expect(result).to.exist;
      expect(result.owner).to.be.a("string");
    });
    
    it("should get all tokens owned by a user", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        tokens: {
          owner: userAddress,
          start_after: null,
          limit: 10
        }
      });
      
      expect(result).to.exist;
      expect(result.tokens).to.be.an("array");
    });
    
    it("should get NFT info", async () => {
      const tokenId = "1"; // Assuming this token exists
      
      const result = await client.queryContractSmart(contractAddress, {
        nft_info: {
          token_id: tokenId
        }
      });
      
      expect(result).to.exist;
      expect(result.token_uri).to.be.a("string");
      expect(result.extension).to.exist;
    });
  });
  
  describe("Admin Functions", () => {
    it("should check if admin role is correctly set", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        is_admin: {
          address: userAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.is_admin).to.be.a("boolean");
    });
  });
}); 