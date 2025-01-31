import { MockServiceRegister } from './fixtures/service.fixture';

describe("Auth - E2E WebAuthn Flow", () => {
  let mockService: MockServiceRegister;

  beforeAll(async () => {
    mockService = new MockServiceRegister(page);
    await mockService.setup();
  });

  it("should register a user successfully", async () => {
    const result = await page.evaluate(async () => {
      const auth = new window.Auth("http://localhost:3000");

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
    });

    if (!result.ok) {
      throw new Error("Registration failed: " + result.error);
    }
    expect(result.data).toBeDefined();
    expect(result.data.success).toBe(true);
  });
});
