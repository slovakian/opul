import { expect, test } from "vite-plus/test";

import { compactTimeRemaining, liveBytes, timeRemaining } from "./model";

const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;

test("live byte counts keep at most three digits and steady decimals", () => {
  expect(liveBytes(0)).toBe("0 B");
  expect(liveBytes(512)).toBe("512 B");
  expect(liveBytes(4.2 * MB)).toBe("4.2 MB");
  expect(liveBytes(42 * MB)).toBe("42.0 MB");
  expect(liveBytes(412 * MB)).toBe("412 MB");
  expect(liveBytes(1.2 * GB)).toBe("1.20 GB");
  expect(liveBytes(12 * GB)).toBe("12.0 GB");
  expect(liveBytes(120 * GB)).toBe("120 GB");
});

test("live byte counts move up a unit instead of printing four digits", () => {
  expect(liveBytes(999.4 * MB)).toBe("999 MB");
  expect(liveBytes(1023.6 * MB)).toBe("1.00 GB");
  expect(liveBytes(99.97 * MB)).toBe("100 MB");
  expect(liveBytes(9.996 * GB)).toBe("10.0 GB");
});

test("compact time left stays short and says nothing while unknown", () => {
  expect(compactTimeRemaining(null)).toBe("—");
  expect(compactTimeRemaining(0)).toBe("0s");
  expect(compactTimeRemaining(45)).toBe("45s");
  expect(compactTimeRemaining(60)).toBe("1m");
  expect(compactTimeRemaining(61)).toBe("2m");
  expect(compactTimeRemaining(12 * 60)).toBe("12m");
  expect(compactTimeRemaining(3599)).toBe("1h 0m");
  expect(compactTimeRemaining(72 * 60)).toBe("1h 12m");
  expect(compactTimeRemaining(52 * 3600)).toBe("2d 4h");
  expect(compactTimeRemaining(200 * 24 * 3600)).toBe("99d+");
});

test("full time left matches the compact value it is read in place of", () => {
  expect(timeRemaining(null)).toBe("Calculating…");
  expect(timeRemaining(45)).toBe("45s left");
  expect(timeRemaining(59.4)).toBe("1m left");
  expect(timeRemaining(3599)).toBe("1h 0m left");
  expect(timeRemaining(7199)).toBe("2h 0m left");
  expect(timeRemaining(52 * 3600)).toBe("2d 4h left");
  expect(timeRemaining(-5)).toBe("0s left");

  for (const seconds of [0, 45, 61, 3599, 3601, 7199, 52 * 3600]) {
    expect(timeRemaining(seconds)).toBe(
      `${compactTimeRemaining(seconds)} left`
    );
  }
});
