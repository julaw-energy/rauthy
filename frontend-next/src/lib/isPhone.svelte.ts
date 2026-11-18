// One shared listener: `phone.value` is true on a screen narrower than Tailwind's `sm` (640px).
class Phone {
    value = $state(false);

    constructor() {
        if (typeof window === 'undefined' || !window.matchMedia) {
            return;
        }
        const mq = window.matchMedia('(max-width: 639px)');
        this.value = mq.matches;
        mq.addEventListener('change', (ev) => (this.value = ev.matches));
    }
}

export const phone = new Phone();

// A touch screen has no hover: explanations that show on hover must open on tap there.
class Touch {
    value = $state(false);

    constructor() {
        if (typeof window === 'undefined' || !window.matchMedia) {
            return;
        }
        const mq = window.matchMedia('(hover: none)');
        this.value = mq.matches;
        mq.addEventListener('change', (ev) => (this.value = ev.matches));
    }
}

export const touch = new Touch();
