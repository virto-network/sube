import './main.js';
import { html, css } from './utils.js';

const buttonTp = html`
  <wa-button>
  </wa-button>
`

const buttonCss = css`
  :host {
    display: inline-block;
    width: 100%;
  }
  wa-button::part(base) {
    font-family: Outfit, sans-serif;
    cursor: pointer;
    width: 100%;
    height: 44px;
    min-height: 44px;
    padding: 12px;
    border-radius: 1000px;
    border: 1px solid #1A1A1A1F;
    opacity: 1;
    background-color: var(--green);
    color: var(--whitesmoke);
    transition: background-color 500ms ease, color 500ms ease;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  wa-button::part(base):hover {
    background-color: var(--lightgreen);
    color: var(--darkslategray);
  }

  wa-button::part(base):focus {
    outline: 2px solid var(--darkslategray);
  }

  :host([variant="secondary"]) > wa-button::part(base):focus {
    outline: 2px solid var(--green);
  }

  :host([variant="secondary"]) > wa-button::part(base) {
    background-color: var(--extra-light-green);
    color: var(--darkslategray);
    border: 1px solid var(--lightgreen);
}

  :host([variant="secondary"]) > wa-button::part(base):hover,
  :host([variant="secondary"]) > wa-button::part(base):focus {
  background-color: var(--whitish-green);
}
`

export class ButtonVirto extends HTMLElement {
  static get TAG() {
    return "virto-button"
  }

  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(buttonTp.content.cloneNode(true));

    const style = document.createElement("style");
    style.textContent = buttonCss;
    this.shadowRoot.appendChild(style);

    this.waButton = this.shadowRoot.querySelector("wa-button");
    this.waButton.textContent = this.getAttribute("label") || "Button";
  }

  static get observedAttributes() {
    return ["label", "variant"]
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'label' && this.shadowRoot) {
        const btn = this.shadowRoot.querySelector('wa-button');
        if (btn) {
            btn.textContent = newValue || "Button";
        }
    }
  }
}

if (!customElements.get(ButtonVirto.TAG)) {
  customElements.define(ButtonVirto.TAG, ButtonVirto)
}