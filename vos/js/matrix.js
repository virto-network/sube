// utilities
const tagFn = fn => (strings, ...parts) => fn(parts.reduce((tpl, value, i) => `${tpl}${strings[i]}${value}`, '').concat(strings[parts.length]))
const html = tagFn(s => new DOMParser().parseFromString(`<template>${s}</template>`, 'text/html').querySelector('template'))
const css = tagFn(s => new CSSStyleSheet().replace(s))
const LANG = navigator.language.split('-')[0]
const i18 = new Proxy({
  en: { user: 'user', pwd: 'password', connect: 'connect', mIdError: 'Invalid user Id', prompt: 'Type here ...', promptEmpty: 'Try to type something' },
  es: { user: 'usuario', pwd: 'contraseña', connect: 'conectar', mIdError: 'Id de usuario invalido', prompt: 'Escribe aquí ...', promptEmpty: 'Escribe algo primero' },
}, { get: (i18, prop) => i18[LANG in i18 ? LANG : 'en'][prop] })

/*
 * MxId is an input field to enter valid Matrix IDs
 */
const userIdTp = html`
<div id="userId" class="input">
  <span>@</span><input id="user" placeholder="${i18.user}" autocapitalize="none" enterkeyhint="next"/>
  <span>:</span><input id="server" list="server-list" />
  <datalist id="server-list"></datalist>
</div>
`
const inputCss = await css`
:host {
  --c-accent: var(--color-accent, #888);
  --c-outline: var(--color-outline, rgba(0, 0, 0, 0.4));
  --in-radius: var(--input-radius, 2px);
  --in-size: var(--input-size, 2.4rem);
}
.input {
  border-radius: var(--in-radius);
  border: 1px solid var(--c-outline);
  box-sizing: border-box;
  display: block;
  font-size: 0.8rem;
  height: var(--in-size);
  outline: none;
  padding: 0.5rem 1ch;
}
::placeholder {
  font-size: 0.8rem;
  color: rgba(0, 0, 0, 0.4)
}
`
const userIdCss = await css`
:host {
  --field-user-width: 4ch;
  --field-server-width: 16ch;
  display: block;
  min-width: 20ch;
}
:host(:focus) .input { border: 1px solid var(--c-accent); }
#userId {
  display: inline-flex;
  overflow: hidden;
  width: 100%;
}
span, input { font-family: monospace; }
span { align-self: center; margin: 0 1px; color: var(--c-outline); }
input {
  border: none;
  padding: 0;
  outline: none;
  font-size: 1.1em;
  overflow: hidden;
}
#user { width: var(--field-user-width); }
#server { width: var(--field-server-width); opacity: 0.9; }
`
export class MxId extends HTMLElement {
  static TAG = 'mx-id'
  static defaultServers = ['matrix.org']
  static formAssociated = true

  #$list
  #$server
  #$user

  #internals
  #serverList = []
  value = ''

  constructor() {
    super()
    let shadow = this.attachShadow({ mode: 'closed', delegatesFocus: true })
    shadow.append(userIdTp.content.cloneNode(true))
    shadow.adoptedStyleSheets = [inputCss, userIdCss]
    this.#internals = this.attachInternals()

    this.#$user = shadow.querySelector('#user')
    this.#$server = shadow.querySelector('#server')
    this.#$list = shadow.querySelector('#server-list')
  }

  connectedCallback() {
    this.#$user.addEventListener('input', this.#adjustUserInput)
    this.#$server.addEventListener('input', this.#adjustServerInput)
    this.#checkInput()
    this.serverList = [...MxId.defaultServers]
  }

  get serverList() { return this.#serverList }
  set serverList(list) {
    if (list.length == 0) return
    this.#serverList = list
    this.#$server.placeholder = list[0]
    list = list.map(server => {
      let opt = document.createElement('option')
      opt.value = server
      return opt
    })
    this.#$list.replaceChildren(...list)
  }

  get mxId() {
    let server = this.#$server.value || this.#serverList[0]
    return parseMxId(`@${this.#$user.value}:${server}`)
  }

  // form-associated custom element
  get form() { return this.#internals.form }
  get name() { return this.getAttribute('name') }
  get type() { return this.localName }
  get validity() { return this.#internals.validity }
  get validationMessage() { return this.#internals.validationMessage }
  get willValidate() { return this.#internals.willValidate }
  checkValidity() { return this.#internals.checkValidity() }
  reportValidity() { return this.#internals.reportValidity() }

  #checkInput() {
    let id = this.mxId?.toString()
    if (id) this.#internals.setValidity({})
    else this.#internals.setValidity({ customError: true }, i18.mIdError, this.#$user)
    this.#internals.setFormValue(id)
  }

  // event handlers
  #adjustUserInput = () => {
    this.#$user.value = this.#$user.value.trim()
    if (this.#$user.value.startsWith('@')) {
      this.#$user.value = this.#$user.value.slice(1)
    }
    if (this.#$user.value.endsWith(':')) {
      this.#$user.value = this.#$user.value.slice(0, -1)
      this.#$server.focus()
    }
    const min = getComputedStyle(this).getPropertyValue('--field-user-width').slice(0, -2)
    this.#$user.style.width = `${Math.max(min, this.#$user.value.length)}ch`
    this.#checkInput()
  }
  #adjustServerInput = () => {
    const min = getComputedStyle(this).getPropertyValue('--field-server-width').slice(0, -2)
    this.#$server.style.width = `${Math.max(min, this.#$server.value.length)}ch`
  }
}
customElements.define(MxId.TAG, MxId)

export function parseMxId(id) {
  if (!id.startsWith('@')) return null
  let [user, server] = id.slice(1).split(':')
  if (user.length == 0) return null
  if (!server) return null
  try { server = new URL(`https://${server}`) } catch { return null }
  return {
    user,
    server,
    toString() { return `@${this.user}:${this.server.host}` }
  }
}

/*
 *
 */
const formTp = html`
<section id="connect" part="connect-form">
  <div id="context"><slot></slot></div>
  <form>
    <label>
      <div class="label">${i18.user}</div>
      <mx-id id="id" name="username" required autocomplete="username webauthn"></mx-id>
    </label>
    <label>
      <div class="label">${i18.pwd}</div>
      <input id="pwd" name="pwd" class="input" type="password" placeholder="${i18.pwd}" required />
    </label>
    <button>${i18.connect}</button>
  </form>
</section>
<section id="connected" hidden><slot name="connected"></slot></section>
`
const formCss = await css`
:host {
  --mx-login-height: 20rem;
  box-sizing: border-box;
  color: var(--colot-txt, #111);
  display: block;
  font-familty: sans;
  padding: 0.4rem 0.6rem;
}
::slotted(p) { margin: 0; }
#connect {
  box-sizing: border-box;
  container-type: size;
  display: flex;
  flex-direction: column;
  height: var(--mx-login-height);
  justify-content: center;
  overflow-y: auto;
  width: 100%;
  &[hidden] { display: none; }
}
#context {
  color: var(--color-txt, #111);
  font-family: sans;
  margin-bottom: 0.5rem;
  width: 100%;
}
form {
  box-sizing: border-box;
  width: 100%;
}
#pwd, #id { margin: 0.2rem 0 0.4rem; width: 100%; }
#pwd { font-family: mono; min-width: 8ch; }
#pwd:focus { border: 1px solid var(--c-accent); }
label {
  color: var(--color-outline, #aaa);
  display: block;
  font-family: sans;
  font-size: 0.8em;
  text-transform: capitalize;
}
button {
  background: var(--c-accent);
  border-radius: var(--in-radius);
  border: none;
  box-sizing: border-box;
  color: white;
  height: var(--in-size);
  margin: 0.3rem 0;
  padding: 0 0.6rem;
  width: 100%;
  font-size: 1em;
}
@container (height < 210px) {
  .label { display: none !important; }
}
@container (width > 500px) and (height < 150px) {
  .label { display: inline !important; }
}
@container (height < 150px) {
  #context { font-size: 0.9em; }
  button { margin: 0 0.3rem; width: fit-content; }
  form { display: flex; }
  label .label { display: inline; margin: auto 0.3rem auto 0; }
  label { display: flex; flex: 1; height: 100%; font-size: 0.9em; margin-right: 0.3rem }
  :is(#id, #pwd.input) { margin: 0; }
}
`
export class MxConnect extends HTMLElement {
  static TAG = 'mx-connect'
  #$form
  #$connect
  #$connected

  constructor() {
    super()
    let shadow = this.attachShadow({ mode: 'open', delegatesFocus: true })
    shadow.append(formTp.content.cloneNode(true))
    shadow.adoptedStyleSheets = [inputCss, formCss]

    this.#$form = shadow.querySelector('form')
    this.#$connect = shadow.querySelector('#connect')
    this.#$connected = shadow.querySelector('#connected')
    this.user = shadow.querySelector('mx-id')
  }

  connectedCallback() {
    this.#$form.addEventListener('submit', this.#onSubmit)
  }

  #onSubmit = (e) => {
    e.preventDefault()
    console.log('connected', new FormData(this.#$form))
    this.#$connect.hidden = true
    this.#$connected.hidden = false
  }
}
customElements.define(MxConnect.TAG, MxConnect)

/*
 *
 */
const promptTp = html`
<aside id="more"><slot name="more"></slot></aside>
<main class="empty" part="prompt">
  <span id="placeholder">${i18.prompt}</span>
  <pre id="input" contenteditable enterkeyhint="send"><br></pre>
  <div id="helpers"><slot></slot></div>
</main>
<div part="action"></div>
`
const emojiTp = html`
<emoji-picker class="popover light" popover="auto"></emoji-picker>
<button type="button" popovertargetaction="toggle">☻</button>
`
const promptCss = await css`
:host {
  --prompt-bg: var(--color-bg, #CCC);
  display: inline-flex;
  position: relative;
  width: 50vw;
}
:host(.code) #input { font-family: monospace; }
:host(:focus) {
  & main { border: 1px solid var(--color-accent); }
  & #placeholder { display: none; }
}
main {
  border: 1px solid transparent;
  box-sizing: border-box;
  display: inline-flex;
  height: 100%;
  min-height: 2.4rem;
  padding: 0.3rem;
  position: relative;
  width: 100%;
  &:not(.empty) #placeholder { display: none; }
}
#placeholder {
  cursor: text;
  color: rgba(0, 0, 0, 0.4);
  font-size: 0.9em;
  flex-shrink: 0;
  margin: auto 0;
}
#input {
  font-family: sans-serif;
  line-height: 1.2em;
  margin: auto 0;
  outline: none;
  overflow-wrap: break-word;
  overflow: hidden;
  white-space: pre-wrap;
  width: 100%;
}
#helpers ::slotted(button) {
  background: none;
  border-radius: 1rem;
  border: none;
  color: var(--color-outline);
  font-size: 1.2em;
  height: 1.8rem;
  line-height: 1rem;
  vertical-align: middle;
  white-space: nowrap;
  width: 1.8rem;
}
#helpers ::slotted(button:is(:hover,:focus)) { color: var(--color-white, #FFF); background: var(--color-outline); }
#helpers ::slotted(button:active) { background: var(--color-accent); }
#helpers ::slotted(.popover) { border: 1px solid var(--color-outline); display: revert; }
#helpers ::slotted(.popover:popover-open) {
  position: absolute;
  inset: unset;
  bottom: 0;
  right: 0;
}
`
export class Prompt extends HTMLElement {
  static TAG = 'mx-prompt'
  static EMOJI_PICKER_URL = 'https://cdn.jsdelivr.net/npm/emoji-picker-element@^1/index.js'
  static formAssociated = true

  #$input
  #internals
  #helpers = []

  constructor() {
    super()
    let shadow = this.attachShadow({ mode: 'closed', delegatesFocus: true })
    shadow.append(promptTp.content.cloneNode(true))
    shadow.adoptedStyleSheets = [promptCss]
    this.#internals = this.attachInternals()

    this.#$input = shadow.querySelector('#input')
    this.#helpers = this.getAttribute('helpers')?.split(' ')
  }

  connectedCallback() {
    this.addEventListener('blur', () => this.#$input.parentElement.classList.toggle('empty', !this.value))
    this.#$input.addEventListener('keypress', this.#enterSubmitOrNewLine)
    this.#$input.addEventListener('input', this.#onInput)
    this.#insert('', true)
    if (this.#helpers?.includes('emoji')) {
      import(Prompt.EMOJI_PICKER_URL).then(() => {
        let [picker, btn] = emojiTp.content.cloneNode(true).children
        btn.popoverTargetElement = picker
        this.append(picker, btn)
      })
      this.addEventListener('emoji-click', ({ target, detail }) => {
        target.hidePopover()
        this.#insert(detail.unicode)
      })
    }
  }

  send() {
    if (!this.value) {
      this.#internals.setValidity({ valueMissing: true }, i18.promptEmpty, this.#$input)
      this.#internals.reportValidity()
      return
    }
    this.#internals.setFormValue(this.value)
    this.#insert('', true)
    this.#internals.form.requestSubmit()
  }

  get value() { return this.#$input.textContent.trim() }

  // form-associated custom element
  get form() { return this.#internals.form }
  get name() { return this.getAttribute('name') }
  get type() { return this.localName }
  get validity() { return this.#internals.validity }
  get validationMessage() { return this.#internals.validationMessage }
  get willValidate() { return this.#internals.willValidate }
  checkValidity() { return this.#internals.checkValidity() }
  reportValidity() { return this.#internals.reportValidity() }

  #insert(txt, replace = false) {
    if (replace) this.#$input.innerHTML = '\n'
    if (txt) {
      let t = this.#$input.lastChild
      t.textContent = `${t.textContent.slice(0, -1)}${txt}\n`
      getSelection().collapse(t, t.textContent.length - 1);
    }
  }

  #debounce = null
  #onInput = e => {
    if (this.#debounce) clearTimeout(this.#debounce)
    this.#debounce = setTimeout(() => this.#internals.setValidity({}), 300)
  }

  #enterSubmitOrNewLine = e => {
    if (e.key == 'Enter') {
      e.preventDefault();
      if (e.shiftKey) this.#insert('\n')
      else this.send()
    }
  }
}
customElements.define(Prompt.TAG, Prompt)

/*
 *
 */
const msgTp = html`
<header part="header"><slot name="header"></slot></header>
<time part="time"></time>
<main id="content"></main>
<aside part="extra"><slot name="extra"></slot></aside>
`
const msgCss = await css`
:host {
  --msg-spacing: 0.5rem;
  display: block;
  padding: var(--msg-spacing);
  position: relative;
}
#content p {
  margin: var(--msg-spacing) 0;
  overflow-wrap: break-word;
  font-family: sans;
}
time {
  color: rgba(0, 0, 0, 0.3);
  font-style: italic;
  font-size: 0.7em;
  position: absolute;
  bottom: calc(var(--msg-spacing) - (var(--msg-spacing) / 2));
  right: var(--msg-spacing);
}
`
export class Message extends HTMLElement {
  static TAG = 'mx-msg'

  #$time
  #$content
  #time = null
  #msg = ''

  constructor() {
    super()
    let shadow = this.attachShadow({ mode: 'closed' })
    shadow.append(msgTp.content.cloneNode(true))
    shadow.adoptedStyleSheets = [msgCss]

    this.#$time = shadow.querySelector('time')
    this.#$content = shadow.querySelector('#content')
  }

  connectedCallback() {
    this.time = this.getAttribute('time') ?? ''
  }

  get message() { return this.#msg }
  set message(msg) {
    this.#msg = msg.trim().replace(/\n{2,}/g, '\n\n')
    this.append(this.#msg)
    this.#$content.append(...this.#msg.split('\n\n')
      .map(txt => {
        txt = txt.split('\n')
        txt = txt.reduce((lines, l, i) => {
          lines.push(l)
          if (i < txt.length - 1) lines.push(document.createElement('br'))
          return lines
        }, [])
        let p = document.createElement('p')
        p.append(...txt)
        return p
      })
    )
  }

  get time() { return this.#time }
  set time(date) {
    date = new Date(date)
    if (isNaN(date)) date = new Date()
    this.#time = date
    this.#$time.dateTime = this.#time.toISOString()
    this.#$time.title = ((Date.now() - this.#time.getTime()) < 24 * 3600 * 1000)
      ? this.#time.toLocaleTimeString() : this.#time.toLocaleString()
    this.#updateTime()
  }

  #updateTime = () => {
    const elapsed = (Date.now() - this.#time.getTime()) / 1000
    requestAnimationFrame(() => {
      this.#$time.textContent = this.#formatElapsed(-elapsed)
      if (elapsed < 60) setTimeout(this.#updateTime, 1000 * 10)
      else if (elapsed < 3600) setTimeout(this.#updateTime, 1000 * 60)
    })
  }

  #formatElapsed(elapsed) {
    const f = new Intl.RelativeTimeFormat(LANG, { numeric: 'auto' })
    const ranges = {
      years: 3600 * 24 * 365,
      months: 3600 * 24 * 30,
      weeks: 3600 * 24 * 7,
      days: 3600 * 24,
      hours: 3600,
      minutes: 60,
      seconds: 1
    }
    for (let key in ranges) {
      if (ranges[key] < Math.abs(elapsed)) {
        const delta = elapsed / ranges[key];
        return f.format(Math.round(delta), key);
      }
    }
    return f.format(-0, 'second')
  }
}
customElements.define(Message.TAG, Message)

