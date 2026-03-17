function mockDisabled(name: string): never {
  throw new Error(`mock feature disabled: ${name}`);
}

export function mockCreateRechargeOrder(..._args: unknown[]): never {
  return mockDisabled("mockCreateRechargeOrder");
}

export function mockGetPlans(..._args: unknown[]): never {
  return mockDisabled("mockGetPlans");
}

export function mockGetRechargeOrder(..._args: unknown[]): never {
  return mockDisabled("mockGetRechargeOrder");
}

export function mockGetWallet(..._args: unknown[]): never {
  return mockDisabled("mockGetWallet");
}

export function mockPurchasePlan(..._args: unknown[]): never {
  return mockDisabled("mockPurchasePlan");
}
