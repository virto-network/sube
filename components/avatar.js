import { html, css } from "./utils.js"

const avatarTp = html`
  <wa-avatar>
    <slot name="icon"></slot>
  </wa-avatar>
`

const avatarCss = css`
  :host {
    display: inline-block;
  }

  wa-avatar {
    --background-color: var(--extra-light-green);
    --text-color: var(--darkslategray);
    --size: 48px;
    font-family: var(--font-primary);
    --border-radius: 50px;
    border-radius: var(--border-radius);
  }

  wa-avatar::part(base) {
    transition: all 0.2s ease-out;
  }

  wa-avatar::part(icon) {
    color: var(--green);
  }

  wa-avatar::part(initials) {
    font-weight: 600;
  }

  wa-avatar::part(image) {
    object-fit: cover;
  }

  :host([shape="rounded-square"]) wa-avatar {
    --border-radius: 12px;
    border-radius: var(--border-radius);
  }

  :host([shape="square"]) wa-avatar {
    --border-radius: 0;
    border-radius: var(--border-radius);
  }

  :host(:hover) wa-avatar::part(base) {
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
  }

  :host([size="small"]) wa-avatar {
    --size: 32px;
  }

  :host([size="large"]) wa-avatar {
    --size: 64px;
  }
`

export class AvatarVirto extends HTMLElement {
  static get TAG() {
    return "virto-avatar";
  }

  static get observedAttributes() {
    return ["image", "label", "initials", "loading", "shape"];
  }

  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(avatarTp.content.cloneNode(true));

    const style = document.createElement("style");
    style.textContent = avatarCss;
    this.shadowRoot.appendChild(style);

    this.waAvatar = this.shadowRoot.querySelector("wa-avatar");
    this.updateAvatar();
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue !== newValue) {
      this.updateAvatar();
    }
  }

  updateAvatar() {
    if (this.waAvatar) {
      ['image', 'label', 'initials', 'loading'].forEach(attr => {
        if (this.hasAttribute(attr)) {
          this.waAvatar.setAttribute(attr, this.getAttribute(attr));
        } else {
          this.waAvatar.removeAttribute(attr);
        }
      });
    }
  }


  connectedCallback() {
    this.waAvatar.addEventListener("wa-error", this.handleError.bind(this));
  }

  disconnectedCallback() {
    this.waAvatar.removeEventListener("wa-error", this.handleError.bind(this));
  }

  handleError(event) {
    this.dispatchEvent(
      new CustomEvent("virto-avatar-error", {
        bubbles: true,
        composed: true,
        detail: event.detail,
      }),
    );
  }
}

if (!customElements.get(AvatarVirto.TAG)) {
  customElements.define(AvatarVirto.TAG, AvatarVirto);
}