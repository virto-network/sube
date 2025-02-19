import 'jest-environment-puppeteer';
import { SubeOptions } from '@virtonetwork/sube';
declare global {
  const page: import('puppeteer').Page;
  const browser: import('puppeteer').Browser;
  const jestPuppeteer: import('jest-puppeteer').Global;
}

import { Auth } from "./src/auth";

declare global {
  interface Window {
    Auth: typeof Auth;
    sube<T>(url: string, options?: SubeOptions): Promise<T>;
  }
}

