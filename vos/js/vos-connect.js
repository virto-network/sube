import { MxConnect } from './matrix.js'
import { Shell } from './sh.js'
// utilities
// const tagFn = fn => (strings, ...parts) => fn(parts.reduce((tpl, value, i) => `${tpl}${strings[i]}${value}`, '').concat(strings[parts.length]))
// const html = tagFn(s => new DOMParser().parseFromString(`<template>${s}</template>`, 'text/html').querySelector('template'))
// const css = tagFn(s => new CSSStyleSheet().replace(s))
const SRC_URL = new URL(import.meta.url)
const PARAMS = SRC_URL.searchParams
const VOS_USER = 'vos-connect-user'

/*
 * The Connect element includes everything you need to interact with VOS
 */
export class Connect extends MxConnect {
  static tag = 'vos-connect'
  static observedAttributes = ['servers']

  sh = new Shell()

  connectedCallback() {
    super.connectedCallback()
    let user = localStorage.getItem(VOS_USER)
    if (user) console.log(user)
    if (!this.deviceId) this.dataset.deviceId = randString(8)
  }

  attributeChangedCallback(name, _a, attr) {
    switch (name) {
      case 'servers': this.user.serverList = attr.split(' ')
        break
    }
  }

  async connect(user) {
    await this.sh.connect(user)
  }

  get deviceId() { return this.dataset.deviceId }
}
if (PARAMS.get('def') != 'no') customElements.define(Connect.tag, Connect)

async function createCredential(mxid, challenge) {
  await navigator.credentials.create({
    publicKey: {
      challenge: [],
      rp: {
        name: 'VOS',
        id: SRC_URL.host,
      },
      user: {
        id: `${mxid}`,
        name: mxid.user,
        displayName: mxid.user,
      },
      pubKeyCredParams: [
        { type: 'public-key', alg: -8 },
        { type: 'public-key', alg: -7 },
      ],
      authenticatorSelection: {
        userVerification: 'required',
        requireResidentKey: true,
        residentKey: 'required',
      },
      attestation: 'none',
    }
  })
}

async function getCredential(challenge) {
  await navigator.credentials.get({
    mediation: 'conditional',
    publicKey: {
      challenge,
      rpId: SRC_URL.host,
    }
  })
}

function randString(length) {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  return Array.from(
    { length },
    () => chars[Math.floor(Math.random() * chars.length)]
  ).join('');
}
