import { Command } from './auth';
import Wallet, { SubeFn, JsWalletFn } from './wallet';

interface Session {
    userId: string;
    createdAt: number;
    address: string;
    wallet: Wallet;
}

interface SavedSession {
    userId: string;
    createdAt: number;
    address: string;
    wallet: {
        address: string;
        mnemonic: string;
        isUnlocked: boolean;
    };
}

export default class SessionManager {
    private sessions: Map<string, Session> = new Map();

    constructor(private subeFn: SubeFn, private JsWalletFn: JsWalletFn) {
        this.loadSessions();
    }

    async create(command: Command, userId: string) {
        const wallet = new Wallet(null, this.subeFn, this.JsWalletFn);
        await wallet.sign(command);

        const address = await wallet.getAddress();

        const session: Session = {
            userId,
            createdAt: Date.now(),
            address,
            wallet
        };

        this.sessions.set(userId, session);
        this.persistSessions();

        return {
            ok: true,
            session
        };
    }

    getWallet(userId: string): Wallet | null {
        const session = this.sessions.get(userId);
        return session?.wallet || null;
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
    }

    private loadSessions() {
        const savedSessions = localStorage.getItem('sessions');
        if (savedSessions) {
            const entries = JSON.parse(savedSessions);
            this.sessions = new Map(
                entries.map(([userId, session]: [string, SavedSession]) => {
                    const wallet = new Wallet(session.wallet.mnemonic, this.subeFn, this.JsWalletFn);
                    return [userId, { ...session, wallet }];
                })
            );
        }
    }
}
