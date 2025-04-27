import { CosmWasmClient, SigningCosmWasmClient, Secp256k1HdWallet } from "cosmwasm";
import { assert, expect } from "chai";

// Enum types from contract
enum JoinType {
  PUBLIC = "PUBLIC",
  PRIVATE = "PRIVATE",
  INVITE_CODE = "INVITE_CODE",
  NFT_GATED = "NFT_GATED",
  MULTI_NFT = "MULTI_NFT",
  ANY_NFT = "ANY_NFT"
}

enum NFTType {
  ERC721 = "ERC721",
  ERC1155 = "ERC1155"
}

enum MemberStatus {
  NONE = "NONE",
  PENDING = "PENDING",
  ACTIVE = "ACTIVE",
  BANNED = "BANNED"
}

describe("TribeController Contract Tests", () => {
  const contractAddress = process.env.TRIBE_CONTROLLER_ADDRESS || "";
  const rpcEndpoint = process.env.RPC_ENDPOINT || "http://localhost:26657";
  
  let client: CosmWasmClient;
  let signingClient: SigningCosmWasmClient;
  let wallet: Secp256k1HdWallet;
  let userAddress: string;
  let testAddress: string = "cosmos1testaddress123456789abcdef";
  
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
  
  describe("Tribe Management", () => {
    let tribeId: number;
    
    it("should create a tribe", async () => {
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_tribe: {
            name: "Test Tribe",
            metadata: "This is a test tribe",
            admins: [userAddress],
            join_type: JoinType.PUBLIC,
            entry_fee: "0",
            nft_requirements: []
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Extract tribe_id from logs
      const tribeIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "tribe_id");
      
      expect(tribeIdLog).to.exist;
      tribeId = parseInt(tribeIdLog?.value || "0");
      expect(tribeId).to.be.a("number");
      expect(tribeId).to.be.greaterThan(0);
    });
    
    it("should update tribe metadata", async () => {
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          update_tribe: {
            tribe_id: tribeId,
            new_metadata: "Updated tribe metadata",
            updated_whitelist: [userAddress, testAddress]
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should update tribe configuration", async () => {
      const nftRequirement = {
        nft_contract: null,
        nft_type: NFTType.ERC721,
        is_mandatory: true,
        min_amount: 1,
        token_ids: []
      };
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          update_tribe_config: {
            tribe_id: tribeId,
            join_type: JoinType.NFT_GATED,
            entry_fee: "10000",
            nft_requirements: [nftRequirement]
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("Tribe Membership", () => {
    let publicTribeId: number;
    let privateTribeId: number;
    
    before(async () => {
      // Create a public tribe
      const publicTribeResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_tribe: {
            name: "Public Test Tribe",
            metadata: "Public tribe for testing",
            admins: [userAddress],
            join_type: JoinType.PUBLIC,
            entry_fee: "0",
            nft_requirements: []
          }
        },
        "auto"
      );
      
      const publicTribeIdLog = publicTribeResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "tribe_id");
      
      publicTribeId = parseInt(publicTribeIdLog?.value || "0");
      
      // Create a private tribe
      const privateTribeResult = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_tribe: {
            name: "Private Test Tribe",
            metadata: "Private tribe for testing",
            admins: [userAddress],
            join_type: JoinType.PRIVATE,
            entry_fee: "0",
            nft_requirements: []
          }
        },
        "auto"
      );
      
      const privateTribeIdLog = privateTribeResult.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "tribe_id");
      
      privateTribeId = parseInt(privateTribeIdLog?.value || "0");
    });
    
    it("should join a public tribe", async () => {
      // Create a second wallet for testing
      const testMnemonic = "your test mnemonic here";
      const testWallet = await Secp256k1HdWallet.fromMnemonic(testMnemonic);
      const [testAccount] = await testWallet.getAccounts();
      const testUserAddress = testAccount.address;
      
      const testSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        testWallet
      );
      
      const result = await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          join_tribe: {
            tribe_id: publicTribeId
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should request to join a private tribe", async () => {
      // Create a second wallet for testing
      const testMnemonic = "your test mnemonic here";
      const testWallet = await Secp256k1HdWallet.fromMnemonic(testMnemonic);
      const [testAccount] = await testWallet.getAccounts();
      const testUserAddress = testAccount.address;
      
      const testSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        testWallet
      );
      
      const result = await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          request_to_join_tribe: {
            tribe_id: privateTribeId
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should approve member", async () => {
      // Using the test address that requested to join
      const testMnemonic = "your test mnemonic here";
      const testWallet = await Secp256k1HdWallet.fromMnemonic(testMnemonic);
      const [testAccount] = await testWallet.getAccounts();
      const testUserAddress = testAccount.address;
      
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          approve_member: {
            tribe_id: privateTribeId,
            member: testUserAddress
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
  });
  
  describe("Tribe Query Functions", () => {
    let tribeId: number;
    
    before(async () => {
      // Create a tribe for testing queries
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_tribe: {
            name: "Query Test Tribe",
            metadata: "Tribe for testing queries",
            admins: [userAddress],
            join_type: JoinType.PUBLIC,
            entry_fee: "0",
            nft_requirements: []
          }
        },
        "auto"
      );
      
      const tribeIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "tribe_id");
      
      tribeId = parseInt(tribeIdLog?.value || "0");
    });
    
    it("should get tribe admin", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_tribe_admin: {
          tribe_id: tribeId
        }
      });
      
      expect(result).to.exist;
      expect(result.admin).to.equal(userAddress);
    });
    
    it("should get tribe whitelist", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_tribe_whitelist: {
          tribe_id: tribeId
        }
      });
      
      expect(result).to.exist;
      expect(result.whitelist).to.be.an("array");
      expect(result.whitelist).to.include(userAddress);
    });
    
    it("should check if address is whitelisted", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        is_address_whitelisted: {
          tribe_id: tribeId,
          user: userAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.result).to.be.true;
    });
    
    it("should get member status", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_member_status: {
          tribe_id: tribeId,
          member: userAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.status).to.equal(MemberStatus.ACTIVE);
    });
    
    it("should get tribe config view", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_tribe_config_view: {
          tribe_id: tribeId
        }
      });
      
      expect(result).to.exist;
      expect(result.config).to.exist;
      expect(result.config.join_type).to.equal(JoinType.PUBLIC);
    });
    
    it("should get member count", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_member_count: {
          tribe_id: tribeId
        }
      });
      
      expect(result).to.exist;
      expect(result.count).to.be.a("number");
      expect(result.count).to.be.greaterThan(0);
    });
  });
  
  describe("Invite Code Functions", () => {
    let tribeId: number;
    let inviteCode: string;
    
    before(async () => {
      // Create a tribe with invite code join type
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_tribe: {
            name: "Invite Code Tribe",
            metadata: "Tribe for testing invite codes",
            admins: [userAddress],
            join_type: JoinType.INVITE_CODE,
            entry_fee: "0",
            nft_requirements: []
          }
        },
        "auto"
      );
      
      const tribeIdLog = result.logs[0].events
        .find(e => e.type === "wasm")
        ?.attributes.find(attr => attr.key === "tribe_id");
      
      tribeId = parseInt(tribeIdLog?.value || "0");
      
      // Generate an invite code
      inviteCode = "TEST_INVITE_CODE_" + Math.floor(Math.random() * 10000);
    });
    
    it("should create an invite code", async () => {
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_invite_code: {
            tribe_id: tribeId,
            code: inviteCode,
            max_uses: 10,
            expiry_time: Math.floor(Date.now() / 1000) + 86400 // 24 hours from now
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should join tribe with invite code", async () => {
      // Create a second wallet for testing
      const testMnemonic = "your test mnemonic here";
      const testWallet = await Secp256k1HdWallet.fromMnemonic(testMnemonic);
      const [testAccount] = await testWallet.getAccounts();
      const testUserAddress = testAccount.address;
      
      const testSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        testWallet
      );
      
      const result = await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          join_tribe_with_code: {
            tribe_id: tribeId,
            invite_code: new TextEncoder().encode(inviteCode)
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
    });
    
    it("should check invite code status", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        get_invite_code_status: {
          tribe_id: tribeId,
          code: inviteCode
        }
      });
      
      expect(result).to.exist;
      expect(result.valid).to.be.true;
      expect(result.remaining_uses).to.be.a("number");
    });
    
    it("should revoke invite code", async () => {
      const result = await signingClient.execute(
        userAddress,
        contractAddress,
        {
          revoke_invite_code: {
            tribe_id: tribeId,
            code: inviteCode
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Verify code is revoked
      const statusResult = await client.queryContractSmart(contractAddress, {
        get_invite_code_status: {
          tribe_id: tribeId,
          code: inviteCode
        }
      });
      
      expect(statusResult.valid).to.be.false;
    });
  });
});

describe("Negative Test Cases", () => {
  let tribeId: number;
  let secondTribeId: number;
  let testWallet: Secp256k1HdWallet;
  let testSigningClient: SigningCosmWasmClient;
  let testUserAddress: string;
  
  // Add client and signing client variables to this suite
  let client: CosmWasmClient;
  let signingClient: SigningCosmWasmClient;
  let wallet: Secp256k1HdWallet;
  let userAddress: string;
  
  const contractAddress = process.env.TRIBE_CONTROLLER_ADDRESS || "";
  const rpcEndpoint = process.env.RPC_ENDPOINT || "http://localhost:26657";
  
  before(async () => {
    // Set up main client
    client = await CosmWasmClient.connect(rpcEndpoint);
    
    // Set up main wallet for admin operations
    const mnemonic = "your test mnemonic here";
    wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic);
    const [firstAccount] = await wallet.getAccounts();
    userAddress = firstAccount.address;
    
    signingClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      wallet
    );
    
    // Create a test wallet for unauthorized user tests
    testWallet = await Secp256k1HdWallet.fromMnemonic(
      "abandon indoor peasant nice address pluck bronze movie inquiry lamp fall story"
    );
    const [testAccount] = await testWallet.getAccounts();
    testUserAddress = testAccount.address;
    
    testSigningClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      testWallet
    );
    
    // Create a test tribe for negative tests
    const result = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        create_tribe: {
          name: "Test Tribe for Negative Tests",
          metadata: "Testing negative scenarios",
          admins: [userAddress],
          join_type: JoinType.PRIVATE,
          entry_fee: "0",
          nft_requirements: []
        }
      },
      "auto"
    );
    
    const tribeIdLog = result.logs[0].events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((attr: any) => attr.key === "tribe_id");
    
    tribeId = parseInt(tribeIdLog?.value || "0");
    
    // Create a second tribe for update/merge tests
    const result2 = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        create_tribe: {
          name: "Second Test Tribe",
          metadata: "For testing updates and merges",
          admins: [userAddress],
          join_type: JoinType.PUBLIC,
          entry_fee: "0",
          nft_requirements: []
        }
      },
      "auto"
    );
    
    const tribeIdLog2 = result2.logs[0].events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((attr: any) => attr.key === "tribe_id");
    
    secondTribeId = parseInt(tribeIdLog2?.value || "0");
  });
  
  it("should fail to create tribe with invalid parameters", async () => {
    try {
      await signingClient.execute(
        userAddress,
        contractAddress,
        {
          create_tribe: {
            name: "", // Empty name should fail
            metadata: "Invalid tribe test",
            admins: [userAddress],
            join_type: JoinType.PUBLIC,
            entry_fee: "0",
            nft_requirements: []
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for empty tribe name");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to update tribe as non-admin", async () => {
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          update_tribe: {
            tribe_id: tribeId,
            metadata: "Unauthorized update attempt"
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for unauthorized update");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to join private tribe directly", async () => {
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          join_tribe: {
            tribe_id: tribeId
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for joining private tribe directly");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to approve/reject member as non-admin", async () => {
    // First, request to join as test user
    await testSigningClient.execute(
      testUserAddress,
      contractAddress,
      {
        request_to_join_tribe: {
          tribe_id: tribeId
        }
      },
      "auto"
    );
    
    // Then try to approve as the same test user (who is not an admin)
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          approve_member: {
            tribe_id: tribeId,
            member: testUserAddress
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for non-admin approving member");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to join already joined tribe", async () => {
    // First, join the public tribe
    await testSigningClient.execute(
      testUserAddress,
      contractAddress,
      {
        join_tribe: {
          tribe_id: secondTribeId
        }
      },
      "auto"
    );
    
    // Then try to join again
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          join_tribe: {
            tribe_id: secondTribeId
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for joining already joined tribe");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to create invite code as non-admin", async () => {
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          create_invite_code: {
            tribe_id: tribeId,
            code: "UNAUTHORIZED_CODE",
            max_uses: 5,
            expiry_time: Math.floor(Date.now() / 1000) + 86400
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for non-admin creating invite code");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to join with invalid invite code", async () => {
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          join_tribe_with_code: {
            tribe_id: tribeId,
            invite_code: new TextEncoder().encode("INVALID_CODE")
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for joining with invalid code");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to query non-existent tribe", async () => {
    try {
      await client.queryContractSmart(contractAddress, {
        get_tribe_config_view: {
          tribe_id: 99999 // Non-existent tribe ID
        }
      });
      
      // Should not reach this point
      expect.fail("Should have thrown an error for non-existent tribe");
    } catch (error) {
      expect(error).to.exist;
    }
  });
});

describe("Tribe Merging", () => {
  let sourceTribeId: number;
  let targetTribeId: number;
  
  // Add client and signing client variables to this suite
  let client: CosmWasmClient;
  let signingClient: SigningCosmWasmClient;
  let wallet: Secp256k1HdWallet;
  let userAddress: string;
  
  const contractAddress = process.env.TRIBE_CONTROLLER_ADDRESS || "";
  const rpcEndpoint = process.env.RPC_ENDPOINT || "http://localhost:26657";
  
  before(async () => {
    // Set up main client
    client = await CosmWasmClient.connect(rpcEndpoint);
    
    // Set up main wallet for admin operations
    const mnemonic = "your test mnemonic here";
    wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic);
    const [firstAccount] = await wallet.getAccounts();
    userAddress = firstAccount.address;
    
    signingClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      wallet
    );
    
    // Create two tribes for merge testing
    const result1 = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        create_tribe: {
          name: "Source Tribe for Merge",
          metadata: "Will be merged into another tribe",
          admins: [userAddress],
          join_type: JoinType.PUBLIC,
          entry_fee: "0",
          nft_requirements: []
        }
      },
      "auto"
    );
    
    const tribeIdLog1 = result1.logs[0].events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((attr: any) => attr.key === "tribe_id");
    
    sourceTribeId = parseInt(tribeIdLog1?.value || "0");
    
    const result2 = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        create_tribe: {
          name: "Target Tribe for Merge",
          metadata: "Will receive members from source tribe",
          admins: [userAddress],
          join_type: JoinType.PUBLIC,
          entry_fee: "0",
          nft_requirements: []
        }
      },
      "auto"
    );
    
    const tribeIdLog2 = result2.logs[0].events
      .find((e: any) => e.type === "wasm")
      ?.attributes.find((attr: any) => attr.key === "tribe_id");
    
    targetTribeId = parseInt(tribeIdLog2?.value || "0");
    
    // Set both tribes as mergeable
    await signingClient.execute(
      userAddress,
      contractAddress,
      {
        set_tribe_mergeable: {
          tribe_id: sourceTribeId,
          is_mergeable: true
        }
      },
      "auto"
    );
    
    await signingClient.execute(
      userAddress,
      contractAddress,
      {
        set_tribe_mergeable: {
          tribe_id: targetTribeId,
          is_mergeable: true
        }
      },
      "auto"
    );
  });
  
  it("should set tribe as mergeable", async () => {
    const result = await client.queryContractSmart(contractAddress, {
      get_tribe_config_view: {
        tribe_id: sourceTribeId
      }
    });
    
    expect(result).to.exist;
    expect(result.config.is_mergeable).to.be.true;
  });
  
  it("should request tribe merge", async () => {
    const result = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        request_tribe_merge: {
          source_tribe_id: sourceTribeId,
          target_tribe_id: targetTribeId
        }
      },
      "auto"
    );
    
    expect(result).to.exist;
    expect(result.logs).to.exist;
  });
  
  it("should get merge request", async () => {
    const result = await client.queryContractSmart(contractAddress, {
      get_merge_request: {
        source_tribe_id: sourceTribeId,
        target_tribe_id: targetTribeId
      }
    });
    
    expect(result).to.exist;
    expect(result.exists).to.be.true;
  });
  
  it("should approve merge request", async () => {
    const result = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        approve_tribe_merge: {
          source_tribe_id: sourceTribeId,
          target_tribe_id: targetTribeId
        }
      },
      "auto"
    );
    
    expect(result).to.exist;
    expect(result.logs).to.exist;
  });
  
  it("should execute merge", async () => {
    const result = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        execute_tribe_merge: {
          source_tribe_id: sourceTribeId,
          target_tribe_id: targetTribeId
        }
      },
      "auto"
    );
    
    expect(result).to.exist;
    expect(result.logs).to.exist;
    
    // Verify source tribe no longer exists
    try {
      await client.queryContractSmart(contractAddress, {
        get_tribe_config_view: {
          tribe_id: sourceTribeId
        }
      });
      
      // Should not reach this point
      expect.fail("Source tribe should no longer exist after merge");
    } catch (error) {
      expect(error).to.exist;
    }
  });
  
  it("should fail to request merge as non-admin", async () => {
    // Create a new tribe for this test
    const result = await signingClient.execute(
      userAddress,
      contractAddress,
      {
        create_tribe: {
          name: "New Source Tribe",
          metadata: "For testing unauthorized merge",
          admins: [userAddress],
          join_type: JoinType.PUBLIC,
          entry_fee: "0",
          nft_requirements: []
        }
      },
      "auto"
    );
    
    const tribeIdLog = result.logs[0].events
      .find(e => e.type === "wasm")
      ?.attributes.find(attr => attr.key === "tribe_id");
    
    const newSourceTribeId = parseInt(tribeIdLog?.value || "0");
    
    // Set as mergeable
    await signingClient.execute(
      userAddress,
      contractAddress,
      {
        set_tribe_mergeable: {
          tribe_id: newSourceTribeId,
          is_mergeable: true
        }
      },
      "auto"
    );
    
    // Create test wallet for unauthorized test
    const testWallet = await Secp256k1HdWallet.fromMnemonic(
      "abandon indoor peasant nice address pluck bronze movie inquiry lamp fall story"
    );
    const [testAccount] = await testWallet.getAccounts();
    const testUserAddress = testAccount.address;
    
    const testSigningClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      testWallet
    );
    
    try {
      await testSigningClient.execute(
        testUserAddress,
        contractAddress,
        {
          request_tribe_merge: {
            source_tribe_id: newSourceTribeId,
            target_tribe_id: targetTribeId
          }
        },
        "auto"
      );
      
      // Should not reach this point
      expect.fail("Should have thrown an error for non-admin requesting merge");
    } catch (error) {
      expect(error).to.exist;
    }
  });
}); 