import variables from './index.scss';

// eslint-disable-next-line import/prefer-default-export
export const CssVariables = {
  PRIMARY_COLOR: variables.primaryColor,
  GREEN_LIGHT_COLOR: variables.greenLightColor,
  RED_COLOR: variables.redColor,
};

// export function getCssVariable(variable: CssVariables): string {
//   return getComputedStyle(document.documentElement).getPropertyValue(variable);
// }
