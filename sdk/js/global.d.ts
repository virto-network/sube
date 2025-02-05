import 'jest-environment-puppeteer';

declare global {
  const page: import('puppeteer').Page;
  const browser: import('puppeteer').Browser;
}

import { Auth } from "./src/auth";

declare global {
  interface Window {
    Auth: typeof Auth;
    signSendAndWait?: <T = any>(tx: any, signer: any) => Promise<T>;
  }
}

