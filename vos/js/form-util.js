const tagFn = fn => (strings, ...parts) => fn(parts.reduce((tpl, value, i) => `${tpl}${strings[i]}${value}`, '').concat(strings[parts.length]))
const html = tagFn(s => new DOMParser().parseFromString(`<template>${s}</template>`, 'text/html').querySelector('template'))
const css = tagFn(s => new CSSStyleSheet().replace(s))

export async function* streamingFormData(selector) {
  const form = document.querySelector(selector)
  if (!form) return
  let lastMsgReady = null
  let lastMsg = new Promise(res => lastMsgReady = res)
  form.addEventListener('submit', e => {
    e.preventDefault()
    lastMsgReady(new FormData(form))
    lastMsg = new Promise(res => lastMsgReady = res)
  })
  while (lastMsg) yield lastMsg
}


const switchTp = html`
<ul id="options" popover="auto"></ul>
<button popovertarget="options" popovertargetaction="toggle"></button>
<main><slot></slot></main>
`
const switchCss = await css`
:host {
  display: flex;
  align-items: center;
}
main { flex: 1; display: flex; }
main ::slotted([name]) { flex: 1; }
#options {
  border: 1px solid var(--color-outline);
  border-radius: 2px;
  padding: 0;
  &:popover-open {
    position: absolute;
    inset: unset;
    bottom: 0;
    left: 0;
  }
}
li {
  padding: 0.5rem;
  &::before { content: attr(data-ic) ' '; }
  &:is(:hover,:focus) {
    background: var(--color-outline, #eee);
    color: white;
    cursor: pointer;
    outline: none;
  }
}
button {
  background: none;
  border-radius: 1rem;
  border: none;
  box-sizing: border-box;
  color: var(--color-outline);
  height: 1.8rem;
  line-height: 1rem;
  margin-right: 0.2rem;
  overflow: hidden;
  vertical-align: middle;
  white-space: nowrap;
  width: 1.8rem;
}
`
export class Switcher extends HTMLElement {
  static TAG = 'input-switcher'
  static formAssociated = true

  #$switchBtn
  #$optionList

  #internals
  #options = {}
  value = null

  constructor() {
    super()
    let shadow = this.attachShadow({ mode: 'open', delegatesFocus: true })
    shadow.append(switchTp.content.cloneNode(true))
    shadow.adoptedStyleSheets = [switchCss]
    this.#internals = this.attachInternals()

    this.#$switchBtn = shadow.querySelector('button')
    this.#$optionList = shadow.querySelector('#options')
  }

  connectedCallback() {
    this.querySelectorAll('template').forEach(tpl => {
      let value = tpl.dataset.value
      if (!value) return
      this.#$optionList.append(this.#initOption(tpl.dataset))
      this.#options[value] = tpl
    })
    this.#$optionList.addEventListener('click', this.#optionSelected)
    this.#$optionList.addEventListener('keypress', this.#optionSelected)
    this.#$optionList.addEventListener('keydown', e => {
      if (e.key == 'ArrowUp' || e.key == 'k') e.target.previousElementSibling?.focus()
      if (e.key == 'ArrowDown' || e.key == 'j') e.target.nextElementSibling?.focus()
    })
    this.select(this.getAttribute('default'))
  }

  select(opt) {
    const options = Object.keys(this.#options)
    if (options.length == 0) return
    opt = options.includes(opt) ? opt : options[0]
    if (this.value == opt) return
    this.value = opt

    this.#internals.setFormValue(this.value)
    this.#$switchBtn.textContent = this.#options[this.value].dataset.ic
    const selection = this.#options[this.value].content.cloneNode(true)
    this.replaceChildren(selection)
  }

  #optionSelected = (e) => {
    if (e.key && e.key != 'Enter') return
    this.select(e.target.dataset.value)
    this.#$optionList.hidePopover()
  }

  #initOption({ option, ic, value }) {
    let li = document.createElement('li')
    li.textContent = option
    li.dataset.ic = ic
    li.dataset.value = value
    li.tabIndex = 1
    return li
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
}
customElements.define(Switcher.TAG, Switcher)
