import BigNumber from 'bignumber.js';
import { EnergyPackageSwapRoutes } from 'src/pages/Energy/types/EnergyPackages';
import { symbol } from 'src/pages/Energy/utils/tokenBuy';

function mapPlan(energyPlan?: EnergyPackageSwapRoutes) {
  let energy = new BigNumber(0);
  let uploads = new BigNumber(0);
  let fuel = new BigNumber(0);
  let symbols = 8;
  let ampers = new BigNumber(0);
  let volts = new BigNumber(0);

  const findItemH = energyPlan?.tokenOut.find(
    (item) => item.denom === symbol[0]
  );
  const findItemA = energyPlan?.tokenOut.find(
    (item) => item.denom === symbol[1]
  );
  const findItemV = energyPlan?.tokenOut.find(
    (item) => item.denom === symbol[2]
  );

  if (findItemH) {
    fuel = new BigNumber(findItemH.amount);
    const base = Math.floor(Math.log10(fuel.toNumber() / 1000000));
    symbols = base < 0 ? 8 : base >= 8 ? 1 : 8 - base;
  }

  if (findItemV) {
    uploads = new BigNumber(findItemV.amount).dp(0, BigNumber.ROUND_FLOOR);
  }

  if (findItemA && findItemV) {
    energy = new BigNumber(findItemA.amount).multipliedBy(findItemV.amount);
    ampers = new BigNumber(findItemA.amount);
    volts = new BigNumber(findItemV.amount);
  }

  return { fuel, uploads, energy, symbols, ampers, volts };
}

export default mapPlan;
