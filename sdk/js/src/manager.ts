import { JsWallet } from '@virtonetwork/libwallet';
import { sube } from '@virtonetwork/sube';

interface Session {
    userId: string;
    createdAt: number;
    address: string;
}

export default class SessionManager {
    private wallet: JsWallet;
    private isUnlocked: boolean = false;
    private sessions: Map<string, Session> = new Map();

    constructor() {
        this.wallet = new JsWallet({ Simple: null });
        this.loadSessions();
    }

    async unlock(): Promise<void> {
        if (!this.isUnlocked) {
            await this.wallet.unlock(null, null);
            this.isUnlocked = true;
        }
    }

    async getAddress(): Promise<string> {
        if (!this.isUnlocked) {
            await this.unlock();
        }
        return this.wallet.getAddress().toHex();
    }

    async createSession(extrinsic: Record<string, any>, userId: string) {
        if (!this.isUnlocked) {
            await this.unlock();
        }

        const subeFn = (typeof window !== "undefined" && window.sube)
            ? window.sube
            : sube;

        const result = await subeFn("https://kreivo.io/pass/authenticate", {
            body: extrinsic,
            from: this.wallet.getAddress().repr,
            sign: (message: Uint8Array) => this.wallet.sign(message)
        });

        if (!result) {
            throw new Error("Session creation failed");
        }

        const address = await this.getAddress();
        const session: Session = {
            userId,
            createdAt: Date.now(),
            address
        };

        this.sessions.set(userId, session);
        this.persistSessions();

        return {
            ok: true,
            session
        };
    }

    getSession(userId: string): Session | undefined {
        return this.sessions.get(userId);
    }

    getAllSessions(): Session[] {
        return Array.from(this.sessions.values());
    }

    removeSession(userId: string): boolean {
        const removed = this.sessions.delete(userId);
        if (removed) {
            this.persistSessions();
        }
        return removed;
    }

    private persistSessions() {
        localStorage.setItem('sessions', JSON.stringify(Array.from(this.sessions.entries())));
        this.persistWallet();
    }

    private loadSessions() {
        const savedSessions = localStorage.getItem('sessions');
        if (savedSessions) {
            this.sessions = new Map(JSON.parse(savedSessions));
        }
    }

    private persistWallet() {
        localStorage.setItem('wallet', JSON.stringify({
            address: this.getAddress(),
        }));
    }
}
