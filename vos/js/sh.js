const timeout = (ms) => new Promise((res) => setTimeout(res, ms));
const timeoutErr = (ms, error) => timeout(ms).then(() => { throw new Error(`timeout ${error}`) })

/*
 * Shell acts as a proxy to the API running in the background worker
 */
export class Shell {
  static worker = null

  #pendingOut = null // promise resolver
  #onMsg = ({ data: out }) => {
    if ('Ok' in out) this.#pendingOut?.resolve(out['Ok'])
    else if ('Err' in out) this.#pendingOut?.reject(out['Err'])
    else console.warn('unexpected', out)
  }
  #resetOutput = () => new Promise((resolve, reject) => {
    this.#pendingOut = { resolve, reject }
  }).then(out => {
    if (out.hasOwnProperty('waitingAuth'))
      throw new ConnectError({ challenge: out.waitingAuth })
    return out
  })

  output = this.#resetOutput()

  constructor() {
    if (!Shell.worker) {
      let params = new URL(import.meta.url).searchParams
      Shell.worker = new Worker(new URL(`vos.js?${params}`, import.meta.url), { type: 'module' })
      Shell.worker.addEventListener('message', this.#onMsg)
    }
  }

  async ping() {
    return this.send({ empty: null })
  }

  async prompt(prompt) {
    return this.send({ prompt })
  }

  async connect(id, credentials) {
    if (!id) throw new ConnectError({ msg: `Mxid ${id}` })
    try { await this.ping() } catch (e) {
      if (e instanceof ConnectError) challenge = e.challenge
      throw e
    }
    credentials = typeof credentials == 'string'
      ? { pwd: { user: `${id}`, pwd: credentials } }
      : { authenticator: credentials }
    return this.send({ auth: [`${id}`, credentials] })
  }

  async* updates() { yield this.output }

  async send(input) {
    Shell.worker.postMessage(input)
    let out = await this.output
    this.output = this.#resetOutput()
    return await Promise.race([this.output, timeoutErr(3000, `sending command`)])
  }
}

class ConnectError extends Error {
  constructor({ msg = 'Not connected', challenge = null }) {
    super(msg)
    this.challenge = challenge
  }
}

