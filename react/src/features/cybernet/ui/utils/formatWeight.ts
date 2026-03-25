export function formatWeightToGrade(value: number, maxWeightsLimit: number): number {
  return parseFloat(((value / maxWeightsLimit) * 10).toFixed(0));
}

export function formatGradeToWeight(value: number, maxWeightsLimit: number): number {
  return parseFloat(((value / 10) * maxWeightsLimit).toFixed(0));
}
