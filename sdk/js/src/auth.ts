import { VError } from "./utils/error";
import { fromBase64, fromBase64Url } from "./utils/base64";
import SessionManager from "./manager";
import { JsWallet } from "@virtonetwork/libwallet";
import { sube } from "@virtonetwork/sube";

export type BaseProfile = {
  id: string;
  name: string;
  displayName: string;
};

export type User<Profile, Metadata extends Record<string, unknown>> = {
  profile: Profile;
  metadata: Metadata;
};

export type Command = {
  url: string;
  body: any;
  hex: string;
};

export default class Auth {
  private sessionManager: SessionManager;
  constructor(
    private readonly baseUrl: string,
    private subeFn: typeof sube,
    private JsWalletFn: typeof JsWallet,
    sessionManagerFactory: () => SessionManager = () => new SessionManager(this.subeFn, this.JsWalletFn)
  ) {
    this.sessionManager = sessionManagerFactory();
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

  async connect(userId: string) {
    const preRes = await fetch(`${this.baseUrl}/pre-connect`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ userId }),
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

    const wallet = this.sessionManager.getWallet(userId);
    if (!wallet) {
      throw new VError("E_CANT_GET_CREDENTIAL", "Credential retrieval failed");
    }

    const sessionPreparationRes = await fetch(`${this.baseUrl}/pre-connect-session`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        id: response.id,
        address: wallet.getAddress()
      }),
    });

    const data = await sessionPreparationRes.json();
    console.log("Post-connect response:", data);

    const sessionResult = await this.sessionManager.create(data.command, userId);

    return {
      ...data,
      ...sessionResult
    };
  }

  async sign(userId: string, command: Command) {
    const wallet = this.sessionManager.getWallet(userId);
    if (!wallet) {
      throw new VError("E_CANT_GET_CREDENTIAL", "Credential retrieval failed");
    }

    const signedExtrinsic = await wallet.sign(command);

    return {
      userId,
      signedExtrinsic,
      originalExtrinsic: command.hex
    };
  }
}


