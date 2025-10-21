<script>
  import MarkdownEditor from '$lib/components/md-editor.svelte';

  /**
   * @type
   * {{ data: {
   *    fileName: string;
   *    content: string;
   * }}}
   */
  let { data } = $props();

  let { fileName, content } = $derived(data);

  let body = $derived(content || '');

  $effect(() => {
    body = content || '';
  });
</script>

<MarkdownEditor {fileName} content={body} />
