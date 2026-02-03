import { assetsBuy } from 'src/pages/Energy/utils/tokenBuy';
import { useAppSelector } from 'src/redux/hooks';
import BuyPackages from '../BuyPackages/BuyPackages';
import SelectPackages from '../SelectPackages/SelectPackages';

console.log('assetsBuy', assetsBuy);

// const selectPackageKey = '10';

function EnergyMain() {
  const { selectPlan } = useAppSelector((state) => state.energy);
  console.log('selectPlan', selectPlan)

  if (selectPlan) {
    return <BuyPackages />;
  }

  return <SelectPackages />;

  // return (
  //   <div>
  //     {energyPackageSwapRoutes &&
  //       energyPackageSwapRoutes.map((item) => {
  //         return (
  //           <div key={item.keyPackage}>
  //             <div>{item.keyPackage}$</div>
  //             <div>
  //               {item.swapInfo.map((itemInfo) => {
  //                 return (
  //                   <div key={itemInfo.swap.tokenOut.denom}>
  //                     <div>{itemInfo.swap.tokenOut.amount}</div>
  //                     <div>
  //                       {getOsmoAssetByDenom(itemInfo.swap.tokenOut.denom)
  //                         ?.symbol || ''}
  //                     </div>
  //                   </div>
  //                 );
  //               })}
  //             </div>
  //             <Button text="energy" onClick={() => onClickSelectPlan(item)} />
  //           </div>
  //         );
  //       })}
  //   </div>
  // );
}

export default EnergyMain;
