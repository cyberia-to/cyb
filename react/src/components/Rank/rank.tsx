import { Link } from 'react-router-dom';
import QuestionBtn from './QuestionBtn/QuestionBtn';

// const gradeColorRank = (rank) => {
//   let rankGradeColor = '#546e7a';

//   switch (rank) {
//     case 1:
//       rankGradeColor = '#ff3d00';
//       break;
//     case 2:
//       rankGradeColor = '#ff9100';
//       break;
//     case 3:
//       rankGradeColor = '#ffea00';
//       break;
//     case 4:
//       rankGradeColor = '#64dd17';
//       break;
//     case 5:
//       rankGradeColor = '#00b0ff';
//       break;
//     case 6:
//       rankGradeColor = '#304ffe';
//       break;
//     case 7:
//       rankGradeColor = '#d500f9';
//       break;
//     default:
//       rankGradeColor = '#546e7a';
//       break;
//   }

//   return rankGradeColor;
// };

function Rank({ rank, hash, ...props }) {
  // const gradeValue = getRankGrade(rank);
  // const color = gradeColorRank(gradeValue.value);

  return (
    <Link to="https://docs.cyb.ai/#/page/cyberank" replace target="_blank">
      <QuestionBtn {...props} />
    </Link>
  );
}

export default Rank;
