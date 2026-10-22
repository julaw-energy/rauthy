<script lang="ts">
    import {CalendarClock, CalendarCheck} from "@lucide/svelte";
    import * as InputGroup from "@/components/ui/input-group/index.js"
    import * as Popover from "$lib/components/ui/popover/index.js";
    import Calendar from "$lib/components/ui/calendar/calendar.svelte";
    import { Button } from "$lib/components/ui/button/index.js";
    import {getLocalTimeZone, now, parseDateTime, toZoned, type DateValue, type ZonedDateTime} from "@internationalized/date";
    import Time from "./time.svelte";

    type SegmentResult = {
        value: string;
        consumed: number;
        complete: boolean;
        valid: boolean;
    };

    type NormalizeResult = {
        value: string;
        accepted: boolean;
    };

    let {
        value = $bindable(),
        label,
        disabled = false,
        required = false,
        minYear = 1900,
        maxYear = new Date().getFullYear() + 100,
        placeholder,
        dateOnly=false
    }: {
        value?: DateValue;
        label: string;
        disabled?: boolean;
        required?: boolean;
        minYear?: number;
        maxYear?: number;
        placeholder?: string;
        dateOnly?: boolean;
    } = $props();


    let widgetValue = $state<DateValue>(now(getLocalTimeZone()))
    let stringValue = $state(dateValueToString(value))
    const complete = $derived(stringValue.length === 19);
    const valid = $derived(isValidDateTime(stringValue));
    let open = $state(false);

    function pad(num: number){ return String(num).padStart(2, "0"); }

    function dateValueToString(dateValue: DateValue | undefined): string {
        if (!dateValue) return ""
        const yyyy = dateValue.year;
        const mm = pad(dateValue.month);
        const dd = pad(dateValue.day);

        // Extract time components safely (in case it's a CalendarDate without time)
        const hh = "hour" in dateValue ? pad(dateValue.hour) : "00";
        const min = "minute" in dateValue ? pad(dateValue.minute) : "00";
        const ss = "second" in dateValue ? pad(dateValue.second) : "00";

        return dateOnly ? `${yyyy}-${mm}-${dd}` : `${yyyy}-${mm}-${dd} ${hh}:${min}:${ss}`;
    }

    function stringToDateValue(dateString: string): ZonedDateTime {
        const isoString = dateString.replace(" ", "T");
        return toZoned(parseDateTime(isoString), getLocalTimeZone());
    }

    function emptySegment(): SegmentResult {
        return {
            value: "",
            consumed: 0,
            complete: false,
            valid: true
        };
    }

    function getDaysInMonth(year: number, month: number): number {
        return new Date(year, month, 0).getDate();
    }

    function normalizeYear(digits: string): SegmentResult {
        const raw = digits.slice(0, 4);

        if (!raw.length) {
            return emptySegment();
        }

        if (raw.length < 4) {
            return {
                value: raw,
                consumed: raw.length,
                complete: false,
                valid: true
            };
        }

        const candidate = Number(raw);

        if (candidate < minYear || candidate > maxYear) {
            /*
             * Reject the fourth digit and retain the first three.
             *
             * Example:
             * 1800 -> 180
             */
            return {
                value: raw.slice(0, 3),
                consumed: 3,
                complete: false,
                valid: false
            };
        }

        return {
            value: raw,
            consumed: 4,
            complete: true,
            valid: true
        };
    }

    function normalizeMonth(digits: string): SegmentResult {
        if (!digits.length) {
            return emptySegment();
        }

        const firstDigit = Number(digits[0]);

        /*
         * A first digit from 2 through 9 can only represent a
         * single-digit month.
         *
         * 2 -> 02
         * 9 -> 09
         */
        if (firstDigit >= 2 && firstDigit <= 9) {
            return {
                value: `0${firstDigit}`,
                consumed: 1,
                complete: true,
                valid: true
            };
        }

        /*
         * 0 and 1 may begin a two-digit month.
         */
        if (digits.length === 1) {
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: true
            };
        }

        const raw = digits.slice(0, 2);
        const candidate = Number(raw);

        if (candidate < 1 || candidate > 12) {
            /*
             * Reject the second digit.
             *
             * 19 -> 1
             * 00 -> 0
             */
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: false
            };
        }

        return {
            value: raw,
            consumed: 2,
            complete: true,
            valid: true
        };
    }

    function normalizeDay(
        digits: string,
        year: number,
        month: number
    ): SegmentResult {
        if (!digits.length) {
            return emptySegment();
        }

        const firstDigit = Number(digits[0]);
        const maximumDay = getDaysInMonth(year, month);

        /*
         * A first digit from 4 through 9 can only represent a
         * single-digit day.
         *
         * 4 -> 04
         * 9 -> 09
         */
        if (firstDigit >= 4 && firstDigit <= 9) {
            return {
                value: `0${firstDigit}`,
                consumed: 1,
                complete: true,
                valid: true
            };
        }

        if (digits.length === 1) {
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: true
            };
        }

        const raw = digits.slice(0, 2);
        const candidate = Number(raw);

        if (candidate < 1 || candidate > maximumDay) {
            /*
             * Reject the second digit.
             *
             * February 30 -> keep 3
             * April 31    -> keep 3
             */
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: false
            };
        }

        return {
            value: raw,
            consumed: 2,
            complete: true,
            valid: true
        };
    }

    function normalizeHour(digits: string): SegmentResult {
        if (!digits.length) {
            return emptySegment();
        }

        const firstDigit = Number(digits[0]);

        /*
         * A first digit from 3 through 9 can only represent a
         * single-digit hour.
         *
         * 7 -> 07
         */
        if (firstDigit >= 3 && firstDigit <= 9) {
            return {
                value: `0${firstDigit}`,
                consumed: 1,
                complete: true,
                valid: true
            };
        }

        if (digits.length === 1) {
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: true
            };
        }

        const raw = digits.slice(0, 2);
        const candidate = Number(raw);

        if (candidate < 0 || candidate > 23) {
            /*
             * Reject the second digit.
             *
             * 29 -> 2
             */
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: false
            };
        }

        return {
            value: raw,
            consumed: 2,
            complete: true,
            valid: true
        };
    }

    function normalizeMinuteOrSecond(
        digits: string
    ): SegmentResult {
        if (!digits.length) {
            return emptySegment();
        }

        const firstDigit = Number(digits[0]);

        /*
         * A first digit from 6 through 9 can only represent a
         * single-digit minute or second.
         *
         * 8 -> 08
         */
        if (firstDigit >= 6 && firstDigit <= 9) {
            return {
                value: `0${firstDigit}`,
                consumed: 1,
                complete: true,
                valid: true
            };
        }

        if (digits.length === 1) {
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: true
            };
        }

        const raw = digits.slice(0, 2);
        const candidate = Number(raw);

        if (candidate < 0 || candidate > 59) {
            /*
             * Reject the second digit.
             *
             * 65 -> 6 has already become 06 before this case,
             * while 59 is accepted.
             */
            return {
                value: digits[0],
                consumed: 1,
                complete: false,
                valid: false
            };
        }

        return {
            value: raw,
            consumed: 2,
            complete: true,
            valid: true
        };
    }

    function normalizeDateTime(rawValue: string): NormalizeResult {
        /*
         * Keep only digits. This also supports pasting either:
         *
         * 20260628095216
         * 2026-06-28 09:52:16
         */
        const digits = rawValue
            .replace(/\D/g, "")
            .slice(0, 14);

        if (!digits.length) {
            return {
                value: "",
                accepted: true
            };
        }

        let cursor = 0;
        let normalized = "";
        let accepted = true;

        const year = normalizeYear(digits.slice(cursor));

        normalized += year.value;
        cursor += year.consumed;
        accepted &&= year.valid;

        if (!year.complete) {
            return {
                value: normalized,
                accepted
            };
        }

        normalized += "-";

        const month = normalizeMonth(digits.slice(cursor));

        normalized += month.value;
        cursor += month.consumed;
        accepted &&= month.valid;

        if (!month.complete) {
            return {
                value: normalized,
                accepted
            };
        }

        normalized += "-";

        const day = normalizeDay(
            digits.slice(cursor),
            Number(year.value),
            Number(month.value)
        );

        normalized += day.value;
        cursor += day.consumed;
        accepted &&= day.valid;

        if (!day.complete) {
            return {
                value: normalized,
                accepted
            };
        }

        if (dateOnly){
            return {
                value: normalized,
                accepted
            };
        }

        normalized += " ";

        const hour = normalizeHour(digits.slice(cursor));

        normalized += hour.value;
        cursor += hour.consumed;
        accepted &&= hour.valid;

        if (!hour.complete) {
            return {
                value: normalized,
                accepted
            };
        }

        normalized += ":";

        const minute = normalizeMinuteOrSecond(
            digits.slice(cursor)
        );

        normalized += minute.value;
        cursor += minute.consumed;
        accepted &&= minute.valid;

        if (!minute.complete) {
            return {
                value: normalized,
                accepted
            };
        }

        normalized += ":";

        const second = normalizeMinuteOrSecond(
            digits.slice(cursor)
        );

        normalized += second.value;
        accepted &&= second.valid;

        return {
            value: normalized,
            accepted
        };
    }

    function isValidDateTime(input: string): boolean {
        const match = dateOnly ? input.match(
            /^(\d{4})-(\d{2})-(\d{2})$/
        ):input.match(
            /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})$/
        );

        if (!match) {
            return false;
        }

        const year = Number(match[1]);
        const month = Number(match[2]);
        const day = Number(match[3]);

        if (year < minYear || year > maxYear) {
            return false;
        }

        if (month < 1 || month > 12) {
            return false;
        }

        if (day < 1 || day > getDaysInMonth(year, month)) {
            return false;
        }

        if(dateOnly){
            return true
        }

        const hour = Number(match[4]);
        const minute = Number(match[5]);
        const second = Number(match[6]);

        if (hour < 0 || hour > 23) {
            return false;
        }

        if (minute < 0 || minute > 59) {
            return false;
        }

        return !(second < 0 || second > 59);
    }

    function emitValidValue(): void {
        value = isValidDateTime(stringValue)? stringToDateValue(stringValue) : undefined
    }

    function moveCaretToEnd(
        input: HTMLInputElement
    ): void {
        requestAnimationFrame(() => {
            const position = input.value.length;

            input.setSelectionRange(position, position);
        });
    }

    function handleInput(event: Event): void {
        const input = event.currentTarget as HTMLInputElement;
        const result = normalizeDateTime(input.value);

        stringValue = result.value;
        input.value = result.value;

        moveCaretToEnd(input);
        emitValidValue();
    }

    function handleKeydown(event: KeyboardEvent): void {
        if (event.key !== "Backspace") {
            return;
        }

        const input = event.currentTarget as HTMLInputElement;
        const selectionStart = input.selectionStart ?? 0;
        const selectionEnd = input.selectionEnd ?? selectionStart;

        /*
         * Let the browser handle a selected range normally.
         */
        if (selectionStart !== selectionEnd) {
            return;
        }

        const previousCharacter =
            input.value[selectionStart - 1];

        const isSeparator =
            previousCharacter === "-" ||
            previousCharacter === " " ||
            previousCharacter === ":";

        if (!isSeparator) {
            return;
        }

        event.preventDefault();

        const before = input.value.slice(
            0,
            Math.max(0, selectionStart - 2)
        );

        const after = input.value.slice(selectionStart);

        const result = normalizeDateTime(before + after);

        stringValue = result.value;
        input.value = result.value;

        moveCaretToEnd(input);
        emitValidValue();
    }

    function handlePaste(event: ClipboardEvent): void {
        const pastedValue =
            event.clipboardData?.getData("text") ?? "";

        if (!pastedValue) {
            return;
        }

        event.preventDefault();

        const input = event.currentTarget as HTMLInputElement;
        const result = normalizeDateTime(pastedValue);

        stringValue = result.value;
        input.value = result.value;

        moveCaretToEnd(input);
        emitValidValue();
    }

    function onOpenChange(opened: boolean){
        if (opened){
            widgetValue = value ?? now(getLocalTimeZone())
        }
        open = opened;
    }

    function saveWidgetValue(){
        value = widgetValue
        stringValue = dateValueToString(value)
        open = false
    }

</script>

<div class="space-y-1">
    <InputGroup.Root class="rounded-md">
        <InputGroup.Addon>
            <InputGroup.Text class="font-bold text-foreground">{label}</InputGroup.Text>
        </InputGroup.Addon>
        <InputGroup.Input
                type="text"
                inputmode="numeric"
                autocomplete="off"
                maxlength={dateOnly? 10 : 19}
                aria-label="Date and time"
                aria-invalid={complete && !valid}
                placeholder={placeholder ?? (dateOnly ? "e.g 1999-01-01" : "e.g 1999-01-01 23:52:08")}
                class="aria-invalid:border-destructive aria-invalid:ring-destructive/20"
                {disabled}
                {required}
                value={stringValue}
                oninput={handleInput}
                onkeydown={handleKeydown}
                onpaste={handlePaste}
        />
        <InputGroup.Addon align="inline-end">
            <Popover.Root {open} onOpenChange={onOpenChange}>
                <Popover.Trigger>
                    {#snippet child({ props })}
                        <Button
                                {...props}
                                variant="ghost"
                                class="size-8 text-primary"
                        >
                            <CalendarClock />
                        </Button>
                    {/snippet}
                </Popover.Trigger>
                <Popover.Content class="w-auto scrollbar-thin overflow-auto max-h-[400px] p-0 gap-1" align="start">
                    <Calendar
                            type="single"
                            captionLayout="dropdown"
                            bind:value={widgetValue}
                    />

                    <div class="flex items-center {dateOnly? "justify-end" : "justify-between"} py-1 px-1">
                        {#if !dateOnly}
                            <Time bind:value={widgetValue}/>
                        {/if}

                        <Button variant="default" class="dark:bg-brand-green dark:text-white" onclick={()=> saveWidgetValue()}>
                            Ok
                            <CalendarCheck />
                        </Button>
                    </div>
                </Popover.Content>
            </Popover.Root>
        </InputGroup.Addon>
    </InputGroup.Root>

    {#if complete && !valid} <p class="text-xs text-destructive">
        Enter a valid date and time between
        {minYear} and {maxYear}. </p>
    {/if}
</div>
