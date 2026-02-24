import { u } from 'unist-builder';
import { visit } from 'unist-util-visit';

const CYBERLINK_REGEX = /~\(([^)]+)\)/g;
const NEURON_REGEX = /@\(([^)]+)\)/g;

const isInsideLink = (_node, ancestors) => {
  return ancestors.some((ancestor) => ancestor.type === 'link');
};

const remarkCybSyntax = () => {
  // const CYBERLINK_REGEX = /~\((.*)\)/g;

  return (tree) => {
    visit(tree, 'text', (node, index, parent) => {
      if (!isInsideLink(node, [])) {
        const children = [];
        let lastIndex = 0;
        let match;

        function addText(text, start, end) {
          if (start < end) children.push(u('text', text.slice(start, end)));
        }

        while ((match = CYBERLINK_REGEX.exec(node.value)) !== null) {
          const [fullMatch, askQuery] = match;
          addText(node.value, lastIndex, match.index);
          children.push(
            u(
              'link-cyber',
              {
                url: `https://cyb.ai/oracle/ask/${askQuery}`,
                name: 'cyberlink',
              },
              [u('text', askQuery)]
            )
          );
          lastIndex = match.index + fullMatch.length;
        }

        while ((match = NEURON_REGEX.exec(node.value)) !== null) {
          const [fullMatch, username] = match;
          addText(node.value, lastIndex, match.index);
          children.push(
            u(
              'link-cyber',
              {
                url: `https://cyb.ai/@${username}`,
                name: 'cyberlink',
              },
              [u('text', username)]
            )
          );
          lastIndex = match.index + fullMatch.length;
        }

        addText(node.value, lastIndex, node.value.length);

        if (children.length > 1 || (children.length === 1 && children[0].type !== 'text')) {
          parent.children.splice(index, 1, ...children);
        }
      }
    });
  };
};

export { remarkCybSyntax, CYBERLINK_REGEX, NEURON_REGEX };
