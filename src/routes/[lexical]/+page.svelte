<script>
  import MarkdownEditor from "$lib/components/md-editor.svelte";
  /**
   * @type
   * {{ data: {
   *    fileName: string;
   *    content: string;
   * }}}
   */
  let { data } = $props();

  const { fileName, content } = $derived(data);

</script>

<div class="max-w-4xl mx-auto">
   <MarkdownEditor fileName={fileName} content={content} />
</div>
