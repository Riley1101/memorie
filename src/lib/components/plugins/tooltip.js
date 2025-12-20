import { tooltipFactory, TooltipProvider } from '@milkdown/kit/plugin/tooltip';

export const selectionLengthTooltip = tooltipFactory('sel-length');

const el = document.createElement('button');
el.className = 'selection-tooltip';
el.style.cssText = `
  position: absolute;
  pointer-events: none;
  background: #1e293b;
  color: #f8fafc;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  z-index: 50;
  box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
`;

export const selectionLengthConfig = (ctx) => {
  ctx.set(selectionLengthTooltip.key, {

    view: (view) => {
      const provider = new TooltipProvider({
        content: el,
        shouldShow: (view) => {
          const { selection } = view.state;
          const isTextSelected = selection.from !== selection.to;
          const isFocused = view.hasFocus();
          return isTextSelected;
        },
      });

      return {
        update: (view, prevState) => {
          provider.update(view, prevState);
          if (!view.state.selection.empty) {
            const { from, to } = view.state.selection;
            const text = view.state.doc.textBetween(from, to);
            el.textContent = `Characters: ${text.length}`;
          }
        },

        destroy: () => {
          provider.destroy();
        },
      };
    },
  });
};
