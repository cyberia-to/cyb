import { Pane, Text } from '@cybercongress/gravity';

function CardTemplate({
  title,
  paddingBottom,
  children,
  marginBottom,
  paddingLeftChild,
  borderBottom,
}) {
  return (
    <Pane
      paddingTop={20}
      paddingBottom={paddingBottom || 0}
      paddingX={20}
      borderRadius={5}
      display="flex"
      flexDirection="column"
      boxShadow="0 0 5px #3ab793"
      marginBottom={marginBottom || 0}
    >
      {title && (
        <Pane paddingX={0} paddingTop={5} paddingBottom={10} borderBottom={borderBottom || 'none'}>
          <Text color="#fff" fontSize="20px" fontWeight="500" lineHeight="1.5">
            {title}
          </Text>
        </Pane>
      )}
      <Pane display="flex" paddingLeft={paddingLeftChild || 0} width="100%" flexDirection="column">
        {children}
      </Pane>
    </Pane>
  );
}

export default CardTemplate;
