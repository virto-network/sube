import { MockServiceRegister } from './fixtures/service.fixture';

describe("Auth - E2E WebAuthn Flow", () => {
  let mockService: MockServiceRegister;
  let mockeSube = async (_url: string, options: any) => {
    return { success: true, extrinsic: options.body };
  }

  beforeAll(async () => {
    mockService = new MockServiceRegister(page);
    await mockService.setup();
  });

  it("should register a user successfully", async () => {
    const result = await page.evaluate(async (subeFn) => {
      const auth = new window.Auth("http://localhost:3000", subeFn);

      const user = {
        profile: {
          id: "123",
          name: "john.doe",
          displayName: "John Doe",
        },
        metadata: { role: "admin" },
      };

      try {
        const res = await auth.register(user);
        return { ok: true, data: res };
      } catch (err) {
        return { ok: false, error: (err as Error).message };
      }
    }, mockeSube);

    if (!result.ok) {
      throw new Error("Registration failed: " + result.error);
    }
    expect(result.data).toBeDefined();
    expect(result.data.success).toBe(true);
  });

  it("should connect a user successfully", async () => {
    const credentialId = mockService.getCredentialId();
    console.log("credentialId", credentialId);

    const result = await page.evaluate(async (credentialId, subeFn) => {
      const auth = new window.Auth("http://localhost:3000", subeFn);

      try {
        const res = await auth.connect(credentialId);
        return {
          ok: true,
          data: res,
          storedKeyring: localStorage.getItem('keyring_pair')
        };
      } catch (err) {
        return { ok: false, error: (err as Error).message };
      }
    }, credentialId, mockeSube);

    if (!result.ok) {
      throw new Error("Connection failed: " + result.error);
    }

    expect(result.data).toBeDefined();
    expect(result.data.success).toBe(true);
  });

  it("should sign an extrinsic successfully", async () => {
    const email = "test@example.com";
    const command = {
      url: "https://kreivo.io/pass/authenticate",
      body: {
        "method": "system",
        "args": {
          "remark": "0x01020304",
        }
      },
      hex: "0x01020304"
    };

    // First, ensure a session is created for the email
    await page.evaluate(async (email, subeFn) => {
      const auth = new window.Auth("http://localhost:3000", subeFn);
      await auth.connect(email);
    }, email, mockeSube);

    const result = await page.evaluate(async (email, command, subeFn) => {
      const auth = new window.Auth("http://localhost:3000", subeFn);

      try {
        const res = await auth.sign(email, command);
        return { ok: true, data: res };
      } catch (err) {
        return { ok: false, error: (err as Error).message };
      }
    }, email, command, mockeSube);

    if (!result.ok) {
      throw new Error("Signing failed: " + result.error);
    }

    expect(result.data).toBeDefined();
    expect(result.data.email).toBe(email);
    expect(result.data.originalExtrinsic).toBe(command.hex);
    expect(result.data.signedExtrinsic).toBeTruthy();
  });
});
