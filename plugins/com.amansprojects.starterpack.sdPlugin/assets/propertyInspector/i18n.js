/**
 * Lightweight i18n for Stream Deck plugin property inspectors.
 * Call initI18n(language) after receiving the inInfo parameter.
 */

const I18N_LOCALES = {
	en: {
		"action": "Action",
		"set": "Set",
		"increase": "Increase",
		"decrease": "Decrease",
		"value": "Value",
		"keyDown": "Key down",
		"keyUp": "Key up",
		"dialRotateACW": "Dial rotate anticlockwise",
		"dialRotateCW": "Dial rotate clockwise",
		"dialDown": "Dial down",
		"dialUp": "Dial up",
		"dialRotate": "Dial rotate",
		"dialRotateHint": "Usages of <code>%d</code> will be substituted with the number of ticks turned, where negative values signify counterclockwise rotation and positive values signify clockwise rotation.",
		"writeToPath": "Write to path",
		"showOnKey": "Show on key",
		"deviceId": "Device ID",
		"profile": "Profile",
		"dialPress": "Dial press",
		"details": "Details",
		"keycodeHelp": "You can find a list of all of the keycodes <a href=\"https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values\" target=\"_blank\">here</a>.",
		"inputSimDetails": "This action simulates pressing a key on the keyboard or mouse. You can choose which event triggers the simulation."
	},
	zh_CN: {
		"action": "动作",
		"set": "设置",
		"increase": "增加",
		"decrease": "减少",
		"value": "数值",
		"keyDown": "按下时",
		"keyUp": "松开时",
		"dialRotateACW": "旋钮逆时针",
		"dialRotateCW": "旋钮顺时针",
		"dialDown": "旋钮按下",
		"dialUp": "旋钮松开",
		"dialRotate": "旋钮旋转",
		"dialRotateHint": "使用 <code>%d</code> 会被替换为旋转刻度数，负值表示逆时针旋转，正值表示顺时针旋转。",
		"writeToPath": "写入路径",
		"showOnKey": "在按键上显示",
		"deviceId": "设备 ID",
		"profile": "配置文件",
		"dialPress": "旋钮按下",
		"details": "详情",
		"keycodeHelp": "所有键码列表可在<a href=\"https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values\" target=\"_blank\">这里</a>查看。",
		"inputSimDetails": "此动作模拟按下键盘或鼠标按键。你可以选择触发模拟的事件类型。"
	}
};

let currentLocale = "en";

// deno-lint-ignore no-unused-vars
function initI18n(language) {
	currentLocale = (language && I18N_LOCALES[language]) ? language : "en";
	applyTranslations();
}

function t(key) {
	const dict = I18N_LOCALES[currentLocale] || I18N_LOCALES.en;
	return dict[key] || I18N_LOCALES.en[key] || key;
}

function applyTranslations() {
	document.querySelectorAll("[data-i18n]").forEach(function(el) {
		const key = el.getAttribute("data-i18n");
		const value = t(key);
		if (el.hasAttribute("data-i18n-html")) {
			el.innerHTML = value;
		} else {
			el.textContent = value;
		}
	});
}
