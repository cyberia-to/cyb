import { SliceState } from 'src/features/passport/passports.redux';

function contains(query: string, list: SliceState) {
  return Object.keys(list)
    .filter((key) => {
      const item = list[key];
      if (item?.data?.extension) {
        const { nickname } = item.data.extension;
        return nickname.toLowerCase().startsWith(query.toLowerCase());
      }
      return false;
    })
    .reduce((obj, key) => {
      return { ...obj, [key]: list[key] };
    }, {});
}

export default contains;
