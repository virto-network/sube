import { Keyring } from '@polkadot/keyring';
import { mnemonicGenerate } from '@polkadot/util-crypto';
import { signSendAndWait } from "./utils/signAndSend";
import { SubmittableExtrinsic } from '@polkadot/api/types';

export class SessionManager {
    private keyring: Keyring;
    private mnemonic: string;
    private pair: any;

    constructor() {
        this.keyring = new Keyring({ type: 'sr25519' });
        this.mnemonic = mnemonicGenerate();
        this.pair = this.keyring.addFromUri(this.mnemonic);
    }

    getAddress(): string {
        return this.pair.address;
    }

    async createSession(extrinsic: SubmittableExtrinsic<"promise">) {
        const signSendAndWaitFn = (typeof window !== "undefined" && window.signSendAndWait)
            ? window.signSendAndWait
            : signSendAndWait;

        const result = await signSendAndWaitFn(extrinsic, this.pair);

        const sessionCreated = (result as any).events.find(
            (record: { event: { method: string; }; }) => record.event.method === "SessionCreated"
        );

        if (!sessionCreated) {
            throw new Error("Session creation failed - SessionCreated event not found");
        }

        this.persistKeyring();

        return {
            ok: true,
        };
    }

    private persistKeyring() {
        localStorage.setItem('keyring_pair', JSON.stringify({
            address: this.pair.address,
            meta: this.pair.meta,
            mnemonic: this.mnemonic
        }));
    }
}
