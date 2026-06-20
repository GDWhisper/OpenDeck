import { derived } from "svelte/store";
import { settings } from "./settings";
import en from "./i18n/en.json";
import zh_CN from "./i18n/zh_CN.json";

type Translations = typeof en;
type NestedKeyOf<T, Prefix extends string = ""> = T extends object
	? {
			[K in keyof T & string]: T[K] extends object
				? NestedKeyOf<T[K], `${Prefix}${K}.`>
				: `${Prefix}${K}`;
		}[keyof T & string]
	: never;

type TranslationKey = NestedKeyOf<Translations>;

const locales: Record<string, typeof en> = { en, zh_CN };

function getNestedValue(obj: Record<string, any>, path: string): string | undefined {
	const keys = path.split(".");
	let current: any = obj;
	for (const key of keys) {
		if (current == null || typeof current !== "object") return undefined;
		current = current[key];
	}
	return typeof current === "string" ? current : undefined;
}

export const t = derived(settings, ($settings) => {
	const locale = $settings?.language ?? "en";
	const dict = locales[locale] ?? en;

	return (key: string, params?: Record<string, string>): string => {
		let value = getNestedValue(dict as unknown as Record<string, any>, key);
		if (value === undefined) {
			value = getNestedValue(en as unknown as Record<string, any>, key);
		}
		if (value === undefined) return key;
		if (params) {
			for (const [k, v] of Object.entries(params)) {
				value = value.replace(new RegExp(`\\{${k}\\}`, "g"), v);
			}
		}
		return value;
	};
});
