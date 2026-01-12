import { setContext, getContext } from "svelte";

export class AttachmentsContext {
	files = $state([]);
	fileInputRef = $state(null);

	constructor(
		 accept,
		 multiple,
		 maxFiles,
		 maxFileSize,
		 onError

	) {;this.accept = accept;this.multiple = multiple;this.maxFiles = maxFiles;this.maxFileSize = maxFileSize;this.onError = onError;}

	openFileDialog = () => {
		this.fileInputRef?.click();
	};

	matchesAccept = (file) => {
		if (!this.accept || this.accept.trim() === "") {
			return true;
		}
		if (this.accept.includes("image/*")) {
			return file.type.startsWith("image/");
		}
		return true;
	};

	add = (files) => {
		let incoming = Array.from(files);
		let accepted = incoming.filter((f) => this.matchesAccept(f));

		if (accepted.length === 0) {
			this.onError?.({
				code: "accept",
				message: "No files match the accepted types.",
			});
			return;
		}

		let withinSize = (f) => (this.maxFileSize ? f.size <= this.maxFileSize : true);
		let sized = accepted.filter(withinSize);

		if (sized.length === 0 && accepted.length > 0) {
			this.onError?.({
				code: "max_file_size",
				message: "All files exceed the maximum size.",
			});
			return;
		}

		let capacity =
			typeof this.maxFiles === "number"
				? Math.max(0, this.maxFiles - this.files.length)
				: undefined;
		let capped = typeof capacity === "number" ? sized.slice(0, capacity) : sized;

		if (typeof capacity === "number" && sized.length > capacity) {
			this.onError?.({
				code: "max_files",
				message: "Too many files. Some were not added.",
			});
		}

		let next = [];
		for (let file of capped) {
			next.push({
				id: crypto.randomUUID(),
				type: "file",
				url: URL.createObjectURL(file),
				mediaType: file.type,
				filename: file.name,
			});
		}

		this.files = [...this.files, ...next];
	};

	remove = (id) => {
		let found = this.files.find((file) => file.id === id);
		if (found?.url) {
			URL.revokeObjectURL(found.url);
		}
		this.files = this.files.filter((file) => file.id !== id);
	};

	clear = () => {
		for (let file of this.files) {
			if (file.url) {
				URL.revokeObjectURL(file.url);
			}
		}
		this.files = [];
	};
}

// ============================================================================
// Provider Context for Global State Management
// ============================================================================

export class TextInputController {
	value = $state("");

	setInput = (newValue) => {
		this.value = newValue;
	};

	clear = () => {
		this.value = "";
	};
}

export class PromptInputController {
	textInput;
	attachments;

	constructor(initialInput = "", accept, multiple) {
		this.textInput = new TextInputController();
		this.textInput.value = initialInput;
		this.attachments = new AttachmentsContext(accept, multiple);
	}
}

const ATTACHMENTS_CONTEXT_KEY = Symbol("attachments");
const PROVIDER_CONTEXT_KEY = Symbol("prompt-input-provider");

export function setAttachmentsContext(context) {
	setContext(ATTACHMENTS_CONTEXT_KEY, context);
}

export function getAttachmentsContext() {
	let context = getContext(ATTACHMENTS_CONTEXT_KEY);
	if (!context) {
		throw new Error("usePromptInputAttachments must be used within a PromptInput");
	}
	return context;
}

// Provider context (optional, for external state management)
export function setPromptInputProvider(controller) {
	setContext(PROVIDER_CONTEXT_KEY, controller);
}

export function getPromptInputProvider() {
	return getContext(PROVIDER_CONTEXT_KEY) || null;
}

export function getPromptInputController() {
	let context = getContext(PROVIDER_CONTEXT_KEY);
	if (!context) {
		throw new Error("getPromptInputController must be used within a PromptInputProvider");
	}
	return context;
}