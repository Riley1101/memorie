import {invoke} from "@tauri-apps/api/core";

/** @type {import('./$types').LayoutLoad} */
export const load = async ({ params }) => {
    const fileName = params.lexical;
    /** @type {string} */
    const content = await invoke("read_file", { name: fileName });
    return {
        fileName,
        content,
    };
};
