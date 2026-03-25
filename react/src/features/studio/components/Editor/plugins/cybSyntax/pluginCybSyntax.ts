import { markRule } from '@milkdown/kit/prose';
import { $command, $inputRule, $markSchema, $remark } from '@milkdown/kit/utils';
import { CYBERLINK_REGEX, NEURON_REGEX, remarkCybSyntax } from './remarkCybSyntax';

const remarkCybSyntaxPlugin = $remark('cybSyntaxPlugin', () => remarkCybSyntax);
const markSchemaCybSyntax = $markSchema('link-cyber', () => ({
  attrs: {
    href: {},
  },
  parseDOM: [
    {
      tag: 'a[href]',
      getAttrs: (dom) => {
        return {
          href: (dom as HTMLElement).getAttribute('href'),
        };
      },
    },
  ],
  toDOM: (mark) => ['a', { href: mark.attrs.href }, 0],
  parseMarkdown: {
    match: (node) => {
      return node.type === 'link-cyber' && node.name === 'cyberlink';
    },
    runner: (state, node, markType) => {
      const attrs = node.url as string;
      console.log('node', node);
      state.openMark(markType, { href: attrs });
      state.next(node.children);
      state.closeMark(markType);
    },
  },
  toMarkdown: {
    match: (mark) => {
      return mark.type.name === 'link-cyber';
    },
    runner: (state, mark) => {
      state.withMark(mark, 'link', undefined, {
        url: mark.attrs.href,
      });
    },
  },
}));

const toggleLinkCommand = $command('UpdateLink', (_ctx) => (payload) => {
  console.log('payload', payload);
  // return toggleMark(directiveNode.type(ctx), payload);
});

const inputRuleAsk = $inputRule((ctx) => {
  return markRule(CYBERLINK_REGEX, markSchemaCybSyntax.type(ctx), {
    updateCaptured: ({ fullMatch, start, end }) =>
      !fullMatch.startsWith('~(')
        ? {
            fullMatch: fullMatch.slice(2, fullMatch.length - 1),
            start: start + 2,
            end: end - 1,
          }
        : {},
    getAttr: (match) => {
      return {
        href: `https://cyb.ai/oracle/ask/${Array.from(match)[1]}`,
      };
    },
  });
});

const inputRuleNeuron = $inputRule((ctx) => {
  return markRule(NEURON_REGEX, markSchemaCybSyntax.type(ctx), {
    updateCaptured: ({ fullMatch, start, end }) => {
      return !fullMatch.startsWith('@(')
        ? {
            fullMatch: fullMatch.slice(2, fullMatch.length - 1),
            start: start + 2,
            end: end - 1,
          }
        : {};
    },
    getAttr: (match) => {
      return {
        href: `https://cyb.ai/@${Array.from(match)[1]}`,
      };
    },
  });
});

export {
  inputRuleNeuron,
  inputRuleAsk,
  toggleLinkCommand,
  markSchemaCybSyntax,
  remarkCybSyntaxPlugin,
};
