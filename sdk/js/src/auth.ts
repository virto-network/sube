import { VError } from "./utils/error";
import { fromBase64, fromBase64Url } from "./utils/base64";
import { SessionManager } from "./manager";

export type BaseProfile = {
  id: string;
  name: string;
  displayName: string;
};

export type User<Profile, Metadata extends Record<string, unknown>> = {
  profile: Profile;
  metadata: Metadata;
};

export default class Auth {
  private baseUrl: string;
  private sessionManager: SessionManager;

  constructor(baseUrl: string, sessionManager?: SessionManager) {
    this.baseUrl = baseUrl;
    this.sessionManager = sessionManager || new SessionManager();
  }

  async register<Profile extends BaseProfile, Metadata extends Record<string, unknown>>(
    user: User<Profile, Metadata>
  ) {
    const preRes = await fetch(`${this.baseUrl}/pre-register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(user),
    });

    const attestation = await preRes.json();
    console.log("Pre-register response:", attestation);

    attestation.publicKey.challenge = fromBase64(attestation.publicKey.challenge);
    attestation.publicKey.user.id = fromBase64(attestation.publicKey.user.id);

    const response = await navigator.credentials.create(attestation);
    console.log("Credential response:", response);

    if (!response) {
      throw new VError("E_CANT_CREATE_CREDENTIAL", "Credential creation failed");
    }

    const postRes = await fetch(`${this.baseUrl}/post-register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id: response.id }),
    });

    const data = await postRes.json();
    console.log("Post-register response:", data);

    return data;
  }

  async connect(credentialId: string) {
    const preRes = await fetch(`${this.baseUrl}/pre-connect`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ credentialId }),
    });

    const assertion = await preRes.json();
    console.log("Connect response:", assertion);

    assertion.publicKey.challenge = fromBase64(assertion.publicKey.challenge);

    if (assertion.publicKey.allowCredentials) {
      for (const desc of assertion.publicKey.allowCredentials) {
        desc.id = fromBase64Url(desc.id);
      }
    }

    const response = await navigator.credentials.get(assertion);
    console.log("Credential response:", response);

    if (!response) {
      throw new VError("E_CANT_GET_CREDENTIAL", "Credential retrieval failed");
    }

    const sessionPreparationRes = await fetch(`${this.baseUrl}/pre-connect-session`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        id: response.id,
        address: this.sessionManager.getAddress()
      }),
    });

    const data = await sessionPreparationRes.json();
    console.log("Post-connect response:", data);

    const sessionResult = await this.sessionManager.createSession(data.extrinsic);

    return {
      ...data,
      ...sessionResult
    };
  }
}


