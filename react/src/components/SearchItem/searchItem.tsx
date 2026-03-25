import { LinksType } from 'src/containers/Search/types';
import Display from '../containerGradient/Display/Display';
import Status, { StatusType } from './status';
import styles from './styles.module.scss';

const gradeColorRank = (grade) => {
  let classColor = 'grey';

  switch (grade) {
    case 1:
    case 2:
    case 3:
    case 4:
      classColor = 'red';
      break;
    case 5:
      classColor = 'green';
      break;
    case 6:
      classColor = 'blue';
      break;
    case 7:
      classColor = 'purple';
      break;
    default:
      classColor = 'grey';
      break;
  }

  return classColor;
};

type Props = {
  status: StatusType;
  grade?: {
    value: number;
  };
  linkType: LinksType;
  children: React.ReactNode;
};

function SearchItem({ status, grade, children, linkType }: Props) {
  let colorRank = 'grey';

  if (grade?.value) {
    colorRank = gradeColorRank(grade.value);
  }

  const item = (
    <Display
      color={colorRank === 'grey' ? 'white' : colorRank}
      sideSaber={linkType === 'to' ? 'left' : linkType === 'from' ? 'right' : undefined}
    >
      <div
        style={{
          display: 'flex',
          width: '100%',
          flex: 1,
          flexDirection: 'column',
          wordBreak: 'break-word',
        }}
      >
        <div
          style={{
            display: 'flex',
            width: '100%',
            justifyContent: 'flex-end',
            alignItems: 'center',
          }}
        >
          <div
            style={{
              width: '100%',
              flex: 1,
            }}
          >
            <div className={styles.containerChildren}>{children}</div>
          </div>
          {/* TODO: Status seems that processed incorrect */}
          {status !== 'completed' && <Status status={status} />}
        </div>
      </div>
    </Display>
  );

  return item;
}

export default SearchItem;
