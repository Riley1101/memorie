import { getContext, setContext } from "svelte";

const MESSAGE_BRANCH_CONTEXT_KEY = Symbol("message-branch-context");

export class MessageBranchController {
	currentBranch = $state(0);
	totalBranches = $state(0);

	setCurrentBranch(branchIndex) {
		if (this.totalBranches <= 0) {
			this.currentBranch = Math.max(0, branchIndex);
			return;
		}

		this.currentBranch = Math.min(Math.max(0, branchIndex), this.totalBranches - 1);
	}

	setTotalBranches(count) {
		this.totalBranches = Math.max(0, count);

		if (this.totalBranches === 0) {
			this.currentBranch = 0;
			return;
		}

		if (this.currentBranch >= this.totalBranches) {
			this.currentBranch = this.totalBranches - 1;
		}
	}

	goToPrevious() {
		if (this.totalBranches <= 1) {
			return;
		}

		this.currentBranch =
			this.currentBranch > 0 ? this.currentBranch - 1 : this.totalBranches - 1;
	}

	goToNext() {
		if (this.totalBranches <= 1) {
			return;
		}

		this.currentBranch =
			this.currentBranch < this.totalBranches - 1 ? this.currentBranch + 1 : 0;
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