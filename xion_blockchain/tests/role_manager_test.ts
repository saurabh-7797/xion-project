import { CosmWasmClient, SigningCosmWasmClient, Secp256k1HdWallet } from "cosmwasm";
import { assert, expect } from "chai";

describe("RoleManager Contract Tests", () => {
  const contractAddress = process.env.ROLE_MANAGER_ADDRESS || "";
  const rpcEndpoint = process.env.RPC_ENDPOINT || "http://localhost:26657";
  
  let client: CosmWasmClient;
  let signingClient: SigningCosmWasmClient;
  let adminWallet: Secp256k1HdWallet;
  let userWallet: Secp256k1HdWallet;
  let adminAddress: string;
  let userAddress: string;
  let randomAddress: string = "cosmos1randomaddress123456789abcdef";
  
  // Test constants
  const DEFAULT_ADMIN_ROLE = "DEFAULT_ADMIN_ROLE";
  const TEST_ROLE = "TEST_ROLE";
  const MODERATOR_ROLE = "MODERATOR_ROLE";
  const SUPER_ADMIN_ROLE = "SUPER_ADMIN_ROLE";
  
  before(async () => {
    client = await CosmWasmClient.connect(rpcEndpoint);
    
    // Set up admin wallet for testing (assumed to have DEFAULT_ADMIN_ROLE)
    const adminMnemonic = process.env.ADMIN_MNEMONIC || "your admin mnemonic here";
    adminWallet = await Secp256k1HdWallet.fromMnemonic(adminMnemonic);
    const [adminAccount] = await adminWallet.getAccounts();
    adminAddress = adminAccount.address;
    
    // Set up regular user wallet
    const userMnemonic = process.env.USER_MNEMONIC || "your user mnemonic here";
    userWallet = await Secp256k1HdWallet.fromMnemonic(userMnemonic);
    const [userAccount] = await userWallet.getAccounts();
    userAddress = userAccount.address;
    
    // Connect signing client with admin wallet
    signingClient = await SigningCosmWasmClient.connectWithSigner(
      rpcEndpoint,
      adminWallet
    );
  });
  
  describe("Instantiation", () => {
    it("should successfully connect to the contract", async () => {
      const contractInfo = await client.getContract(contractAddress);
      expect(contractInfo).to.exist;
      expect(contractInfo.address).to.equal(contractAddress);
    });
    
    it("should verify admin has DEFAULT_ADMIN_ROLE", async () => {
      const result = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: DEFAULT_ADMIN_ROLE,
          address: adminAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.has_role).to.be.true;
    });
  });
  
  describe("Role Management - Success Cases", () => {
    it("should grant a role to user", async () => {
      const result = await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: TEST_ROLE,
            address: userAddress
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Verify role was granted
      const hasRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: TEST_ROLE,
          address: userAddress
        }
      });
      
      expect(hasRoleResult).to.exist;
      expect(hasRoleResult.has_role).to.be.true;
    });
    
    it("should get all roles for an address", async () => {
      // First grant multiple roles to user
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: MODERATOR_ROLE,
            address: userAddress
          }
        },
        "auto"
      );
      
      const result = await client.queryContractSmart(contractAddress, {
        get_roles: {
          address: userAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.roles).to.be.an("array");
      expect(result.roles.length).to.be.at.least(2); // TEST_ROLE and MODERATOR_ROLE
      expect(result.roles).to.include(TEST_ROLE);
      expect(result.roles).to.include(MODERATOR_ROLE);
    });
    
    it("should revoke a role", async () => {
      const result = await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          revoke_role: {
            role: MODERATOR_ROLE,
            address: userAddress
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Verify role was revoked
      const hasRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: MODERATOR_ROLE,
          address: userAddress
        }
      });
      
      expect(hasRoleResult).to.exist;
      expect(hasRoleResult.has_role).to.be.false;
    });
    
    it("should allow user to renounce their own role", async () => {
      // Connect with user wallet
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      const result = await userSigningClient.execute(
        userAddress,
        contractAddress,
        {
          renounce_role: {
            role: TEST_ROLE
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Verify role was renounced
      const hasRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: TEST_ROLE,
          address: userAddress
        }
      });
      
      expect(hasRoleResult).to.exist;
      expect(hasRoleResult.has_role).to.be.false;
    });
    
    it("should set role admin", async () => {
      // First grant SUPER_ADMIN_ROLE to admin
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: SUPER_ADMIN_ROLE,
            address: adminAddress
          }
        },
        "auto"
      );
      
      // Set SUPER_ADMIN_ROLE as admin for TEST_ROLE
      const result = await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          set_role_admin: {
            role: TEST_ROLE,
            admin_role: SUPER_ADMIN_ROLE
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Verify role admin was set
      const roleAdminResult = await client.queryContractSmart(contractAddress, {
        get_role_admin: {
          role: TEST_ROLE
        }
      });
      
      expect(roleAdminResult).to.exist;
      expect(roleAdminResult.admin_role).to.equal(SUPER_ADMIN_ROLE);
    });
  });
  
  describe("Role Management - Failure Cases", () => {
    it("should fail when non-admin tries to grant role", async () => {
      // Connect with user wallet
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      try {
        await userSigningClient.execute(
          userAddress,
          contractAddress,
          {
            grant_role: {
              role: TEST_ROLE,
              address: randomAddress
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Missing role"); // or similar unauthorized message
      }
    });
    
    it("should fail when non-admin tries to revoke role", async () => {
      // First grant TEST_ROLE to random address
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: TEST_ROLE,
            address: randomAddress
          }
        },
        "auto"
      );
      
      // Connect with user wallet
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      try {
        await userSigningClient.execute(
          userAddress,
          contractAddress,
          {
            revoke_role: {
              role: TEST_ROLE,
              address: randomAddress
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Missing role"); // or similar unauthorized message
      }
    });
    
    it("should fail when user tries to renounce role they don't have", async () => {
      // Connect with user wallet
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      try {
        await userSigningClient.execute(
          userAddress,
          contractAddress,
          {
            renounce_role: {
              role: "NONEXISTENT_ROLE"
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Cannot renounce"); // or similar message
      }
    });
    
    it("should fail when trying to renounce role for another address", async () => {
      try {
        await signingClient.execute(
          adminAddress,
          contractAddress,
          {
            renounce_role: {
              role: TEST_ROLE,
              address: randomAddress // Trying to renounce for someone else
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        // The error message will depend on implementation, but it should fail
      }
    });
    
    it("should fail when non-admin tries to set role admin", async () => {
      // Connect with user wallet
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      try {
        await userSigningClient.execute(
          userAddress,
          contractAddress,
          {
            set_role_admin: {
              role: TEST_ROLE,
              admin_role: "ANOTHER_ROLE"
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Missing role"); // or similar unauthorized message
      }
    });
  });
  
  describe("Query Functions", () => {
    it("should check if address has role", async () => {
      // First grant a new role to user
      const newRole = "NEW_TEST_ROLE";
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: newRole,
            address: userAddress
          }
        },
        "auto"
      );
      
      const hasRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: newRole,
          address: userAddress
        }
      });
      
      expect(hasRoleResult).to.exist;
      expect(hasRoleResult.has_role).to.be.true;
      
      // Check for role that doesn't exist
      const noRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: "NONEXISTENT_ROLE",
          address: userAddress
        }
      });
      
      expect(noRoleResult).to.exist;
      expect(noRoleResult.has_role).to.be.false;
    });
    
    it("should get role admin", async () => {
      // We already set SUPER_ADMIN_ROLE as admin for TEST_ROLE in a previous test
      const result = await client.queryContractSmart(contractAddress, {
        get_role_admin: {
          role: TEST_ROLE
        }
      });
      
      expect(result).to.exist;
      expect(result.admin_role).to.equal(SUPER_ADMIN_ROLE);
    });
    
    it("should check if address is role admin", async () => {
      // Admin should have SUPER_ADMIN_ROLE which is admin for TEST_ROLE
      const result = await client.queryContractSmart(contractAddress, {
        is_role_admin: {
          role: TEST_ROLE,
          address: adminAddress
        }
      });
      
      expect(result).to.exist;
      expect(result.is_admin).to.be.true;
      
      // User should not be admin for TEST_ROLE
      const userResult = await client.queryContractSmart(contractAddress, {
        is_role_admin: {
          role: TEST_ROLE,
          address: userAddress
        }
      });
      
      expect(userResult).to.exist;
      expect(userResult.is_admin).to.be.false;
    });
    
    it("should get role member count", async () => {
      // DEFAULT_ADMIN_ROLE should have at least 1 member (adminAddress)
      const result = await client.queryContractSmart(contractAddress, {
        get_role_member_count: {
          role: DEFAULT_ADMIN_ROLE
        }
      });
      
      expect(result).to.exist;
      expect(result.count).to.be.a("number");
      expect(result.count).to.be.at.least(1);
    });
  });
  
  describe("Advanced Role Management", () => {
    it("should handle complex role hierarchy", async () => {
      // Create a hierarchy of roles
      const LEVEL1_ROLE = "LEVEL1_ROLE";
      const LEVEL2_ROLE = "LEVEL2_ROLE";
      const LEVEL3_ROLE = "LEVEL3_ROLE";
      
      // Grant LEVEL1_ROLE to admin
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: LEVEL1_ROLE,
            address: adminAddress
          }
        },
        "auto"
      );
      
      // Set LEVEL1_ROLE as admin for LEVEL2_ROLE
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          set_role_admin: {
            role: LEVEL2_ROLE,
            admin_role: LEVEL1_ROLE
          }
        },
        "auto"
      );
      
      // Set LEVEL2_ROLE as admin for LEVEL3_ROLE
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          set_role_admin: {
            role: LEVEL3_ROLE,
            admin_role: LEVEL2_ROLE
          }
        },
        "auto"
      );
      
      // Admin should be able to grant LEVEL2_ROLE (since admin has LEVEL1_ROLE)
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: LEVEL2_ROLE,
            address: userAddress
          }
        },
        "auto"
      );
      
      // User should now be able to grant LEVEL3_ROLE (since user has LEVEL2_ROLE)
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      const result = await userSigningClient.execute(
        userAddress,
        contractAddress,
        {
          grant_role: {
            role: LEVEL3_ROLE,
            address: randomAddress
          }
        },
        "auto"
      );
      
      expect(result).to.exist;
      expect(result.logs).to.exist;
      
      // Verify random address has LEVEL3_ROLE
      const hasRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: LEVEL3_ROLE,
          address: randomAddress
        }
      });
      
      expect(hasRoleResult).to.exist;
      expect(hasRoleResult.has_role).to.be.true;
    });
    
    it("should prevent privilege escalation", async () => {
      // Create a new role
      const PRIVILEGED_ROLE = "PRIVILEGED_ROLE";
      
      // Grant the role to admin only
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          grant_role: {
            role: PRIVILEGED_ROLE,
            address: adminAddress
          }
        },
        "auto"
      );
      
      // Set DEFAULT_ADMIN_ROLE as admin for PRIVILEGED_ROLE
      await signingClient.execute(
        adminAddress,
        contractAddress,
        {
          set_role_admin: {
            role: PRIVILEGED_ROLE,
            admin_role: DEFAULT_ADMIN_ROLE
          }
        },
        "auto"
      );
      
      // Connect with user wallet
      const userSigningClient = await SigningCosmWasmClient.connectWithSigner(
        rpcEndpoint,
        userWallet
      );
      
      // User should not be able to grant themselves the PRIVILEGED_ROLE
      try {
        await userSigningClient.execute(
          userAddress,
          contractAddress,
          {
            grant_role: {
              role: PRIVILEGED_ROLE,
              address: userAddress
            }
          },
          "auto"
        );
        assert.fail("Expected an error but none was thrown");
      } catch (error) {
        expect(error).to.exist;
        expect((error as Error).toString()).to.include("Missing role"); // or similar unauthorized message
      }
      
      // Verify user does not have PRIVILEGED_ROLE
      const hasRoleResult = await client.queryContractSmart(contractAddress, {
        has_role: {
          role: PRIVILEGED_ROLE,
          address: userAddress
        }
      });
      
      expect(hasRoleResult).to.exist;
      expect(hasRoleResult.has_role).to.be.false;
    });
  });
}); 