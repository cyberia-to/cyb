import { Colors } from 'src/components/containerGradient/types';
import ghostIcon from '../ui/pages/SelectPackages/images/ghost.png';
import smartIcon from '../ui/pages/SelectPackages/images/smart.png';
import prodigyIcon from '../ui/pages/SelectPackages/images/prodigy.png';
import geniusIcon from '../ui/pages/SelectPackages/images/genius.png';
import { EnergyPackages } from '../constants';

export interface Feature {
  label: string;
  subLabel: string;
}

export interface Plan {
  name: string;
  color: Colors;
  price: string;
  icon: string;
  features: boolean[];
  symbols: number;
  uploads: number;
  fuel: number;
  energy: number;
  influence: number;
  ampers: number;
  volts: number;
}

export const features: Feature[] = [
  { label: 'brain surf', subLabel: 'search for free' },
  { label: 'private pins', subLabel: 'unlimited' },
  { label: 'public upload', subLabel: 'cyberlinks daily' },
  { label: '.moon citizenship', subLabel: 'afford cool short name' },
  { label: 'fuel', subLabel: 'main token, liquid' },
  { label: 'energy', subLabel: 'your power, A x V = kW' },
  { label: 'influence', subLabel: 'your content visibility' },
  { label: '3 free to use aips', subLabel: '' },
  { label: 'all powered aips', subLabel: '' },
];

export const plans: Plan[] = [
  {
    name: 'ghost',
    color: Colors.WHITE,
    price: 'free',
    icon: ghostIcon,
    features: [true, true, false, false, false, false, false, true, false],
    symbols: 8,
    uploads: 0,
    fuel: 0,
    energy: 0,
    influence: 0,
    ampers: 0,
    volts: 0,
  },
  {
    name: 'smart',
    color: Colors.GREEN,
    price: EnergyPackages[0],
    icon: smartIcon,
    features: [true, true, true, true, true, true, true, true, false],
    symbols: 5,
    uploads: 80,
    fuel: 2,
    energy: 4,
    influence: 1,
    ampers: 0,
    volts: 0,
  },
  {
    name: 'prodigy',
    color: Colors.BLUE,
    price: EnergyPackages[1],
    icon: prodigyIcon,
    features: [true, true, true, true, true, true, true, true, true],
    symbols: 4,
    uploads: 800,
    fuel: 20,
    energy: 1,
    influence: 7,
    ampers: 0,
    volts: 0,
  },
  {
    name: 'genius',
    color: Colors.PURPLE,
    price: EnergyPackages[2],
    icon: geniusIcon,
    features: [true, true, true, true, true, true, true, true, true],
    symbols: 3,
    uploads: 8000,
    fuel: 200,
    energy: 11,
    influence: 19,
    ampers: 0,
    volts: 0,
  },
];
