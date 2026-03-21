import type { NavigateFunction } from 'react-router-dom';

let navigate: NavigateFunction;

export const setNavigate = (nav: NavigateFunction) => {
  navigate = nav;
};

export const getNavigate = () => navigate;
