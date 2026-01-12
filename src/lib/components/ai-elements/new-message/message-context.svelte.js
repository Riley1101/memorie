import { getContext, setContext } from "svelte";

// Message Branch Context Types

const MESSAGE_BRANCH_CONTEXT_KEY = Symbol("message-branch-context");

// MessageBranch Class for state management
export class MessageBranchClass {
	currentBranch = $state(0);
	branches = $state([]);
	 _branchCount = $state(0);

	constructor(defaultBranch = 0) {
		this.currentBranch = defaultBranch;
	}

	get totalBranches() {
		// Use explicit count if set, otherwise fall back to branches array length
		return this._branchCount > 0 ? this._branchCount : this.branches.length;
	}

	setBranches(newBranches) {
		this.branches = newBranches;
		this._branchCount = newBranches.length;
	}

	setBranchCount(count) {
		this._branchCount = count;
	}

	goToPrevious() {
		const total = this.totalBranches;
		const newBranch = this.currentBranch > 0 ? this.currentBranch - 1 : total - 1;
		this.currentBranch = newBranch;
	}

	goToNext() {
		const total = this.totalBranches;
		const newBranch = this.currentBranch < total - 1 ? this.currentBranch + 1 : 0;
		this.currentBranch = newBranch;
	}
}

export function setMessageBranchContext(context) {
	return setContext(MESSAGE_BRANCH_CONTEXT_KEY, context);
}

export function getMessageBranchContext() {
	const context = getContext(MESSAGE_BRANCH_CONTEXT_KEY);

	if (!context) {
		throw new Error("MessageBranch components must be used within MessageBranch");
	}

	return context;
}

// Message Role Type