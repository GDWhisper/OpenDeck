<script lang="ts">
	import ClockClockwise from "phosphor-svelte/lib/ClockClockwise";
	import ClockCounterClockwise from "phosphor-svelte/lib/ClockCounterClockwise";
	import Gear from "phosphor-svelte/lib/Gear";
	import Scroll from "phosphor-svelte/lib/Scroll";
	import Star from "phosphor-svelte/lib/Star";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { settings } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";
	import { t } from "$lib/i18n";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { message } from "@tauri-apps/plugin-dialog";

	let showPopup: boolean;
	let buildInfo: string;
	(async () => buildInfo = await invoke("get_build_info"))();

	listen("device_brightness", ({ payload }: { payload: { action: string; value: number } }) => {
		if (!$settings) return;
		let value = $settings.brightness;
		switch (payload.action) {
			case "increase":
				value += payload.value;
				break;
			case "decrease":
				value -= payload.value;
				break;
			default:
				value = payload.value;
				break;
		}
		$settings.brightness = Math.max(0, Math.min(100, value));
	});

	async function backupConfig() {
		await message($t("settings.backupPrompt"), { title: $t("settings.backupPromptTitle") });
		if (await invoke("backup_config_directory")) {
			await message($t("settings.backupSuccess"), { title: $t("settings.backupSuccessTitle") });
		}
	}

	async function restoreConfig() {
		await message($t("settings.restorePrompt"), { title: $t("settings.restorePromptTitle") });
		await invoke("restore_config_directory");
	}
</script>

<button
	class="px-3 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
	on:click={() => showPopup = true}
>
	{$t("settings.title")}
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showPopup = false;
	}}
/>

<Popup show={showPopup} label={$t("settings.title")}>
	<button class="mr-2 my-1 float-right text-xl text-neutral-300" on:click={() => showPopup = false} aria-label={$t("common.close")}>✕</button>
	<h2 class="m-2 font-semibold text-xl text-neutral-300">{$t("settings.title")}</h2>
	{#if $settings}
		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-language" class="text-neutral-400">{$t("settings.language")}</label>
			<div class="select-wrapper">
				<select bind:value={$settings.language} class="w-32" id="settings-language">
					<option value="en">English</option>
					<option value="es">Español</option>
					<option value="zh_CN">中文</option>
					<option value="fr">Français</option>
					<option value="de">Deutsch</option>
					<option value="ja">日本語</option>
					<option value="ko">韓国語</option>
				</select>
			</div>
			<Tooltip>
				{$t("settings.languageTooltip")}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-brightness" class="text-neutral-400">{$t("settings.brightness")}</label>
			<input type="range" min="0" max="100" bind:value={$settings.brightness} id="settings-brightness" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-sleep_timeout_minutes" class="text-neutral-400">{$t("settings.sleepTimeout")}</label>
			<input type="number" min="0" bind:value={$settings.sleep_timeout_minutes} class="w-12 px-1 text-neutral-300 border border-neutral-600 rounded-lg" id="settings-sleep_timeout_minutes" />
			<span class="text-neutral-400">{$t("settings.sleepTimeoutUnit")}</span>
			<Tooltip>{$t("settings.sleepTimeoutTooltip")}</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-rotation" class="text-neutral-400">{$t("settings.rotation")}</label>
			<input type="range" min="0" max="270" step="90" bind:value={$settings.rotation} id="settings-rotation" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-background" class="text-neutral-400">{$t("settings.runInBackground")}</label>
			<input type="checkbox" bind:checked={$settings.background} id="settings-background" />
			<Tooltip>{$t("settings.runInBackgroundTooltip", { name: PRODUCT_NAME })}</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-autolaunch" class="text-neutral-400">{$t("settings.startAtLogin")}</label>
			<input type="checkbox" bind:checked={$settings.autolaunch} id="settings-autolaunch" />
			<Tooltip>
				{$t("settings.startAtLoginTooltip", { name: PRODUCT_NAME })}
				{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
					<br />
					{$t("settings.startAtLoginFlatpak", { name: PRODUCT_NAME })}
				{/if}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-updatecheck" class="text-neutral-400">{$t("settings.checkForUpdates")}</label>
			<input type="checkbox" bind:checked={$settings.updatecheck} id="settings-updatecheck" />
		</div>

		{#if !buildInfo?.split("</summary>")[0]?.includes("windows")}
			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-separatewine" class="text-neutral-400">{$t("settings.separateWinePrefixes")}</label>
				<input type="checkbox" bind:checked={$settings.separatewine} id="settings-separatewine" />
				<Tooltip>
					{$t("settings.separateWinePrefixesTooltip", { name: PRODUCT_NAME })}
				</Tooltip>
			</div>
		{/if}

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-developer" class="text-neutral-400">{$t("settings.developerMode")}</label>
			<input type="checkbox" bind:checked={$settings.developer} id="settings-developer" />
			<Tooltip>
				{$t("settings.developerModeTooltip")}
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-disableelgato" class="text-neutral-400">{$t("settings.disableElgatoDiscovery")}</label>
			<input type="checkbox" bind:checked={$settings.disableelgato} id="settings-disableelgato" />
			<Tooltip>{$t("settings.disableElgatoDiscoveryTooltip")}</Tooltip>
		</div>
	{/if}

	<div class="ml-2">
		<div class="flex flex-row my-3 space-x-2">
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => backupConfig()}
			>
				<ClockCounterClockwise class="mr-1" />
				{$t("settings.backupConfig")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => restoreConfig()}
			>
				<ClockClockwise class="mr-1" />
				{$t("settings.restoreConfig")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => invoke("open_config_directory")}
			>
				<Gear class="mr-1" />
				{$t("settings.openConfig")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => invoke("open_log_directory")}
			>
				<Scroll class="mr-1" />
				{$t("settings.openLogs")}
			</button>
		</div>

		<span class="text-xs text-neutral-400">
			{@html buildInfo}
		</span>

		<div class="absolute bottom-6 flex flex-row items-center text-sm text-neutral-400">
			<span class="mr-1">
				{$t("settings.pleaseLeave")}
				<button on:click={() => invoke("open_url", { url: "https://github.com/GDWhisper/OpenDeck-Win" })} class="underline">{$t("settings.starOnGitHub")}</button>
			</span>
			<Star weight="fill" fill="yellow" />
			<span class="ml-1">{$t("settings.forMyWork")}</span>
		</div>
	</div>
</Popup>
