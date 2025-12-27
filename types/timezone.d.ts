// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

export {};

/**
 * Minimal Intl.DateTimeFormat implementation for timezone support.
 * Enables dayjs and similar libraries to work with timezone conversions.
 */
declare global {
  namespace Intl {
    interface DateTimeFormatOptions {
      localeMatcher?: "best fit" | "lookup";
      weekday?: "long" | "short" | "narrow";
      era?: "long" | "short" | "narrow";
      year?: "numeric" | "2-digit";
      month?: "numeric" | "2-digit" | "long" | "short" | "narrow";
      day?: "numeric" | "2-digit";
      hour?: "numeric" | "2-digit";
      minute?: "numeric" | "2-digit";
      second?: "numeric" | "2-digit";
      timeZoneName?: "short" | "long" | "shortOffset" | "longOffset";
      formatMatcher?: "best fit" | "basic";
      hour12?: boolean;
      timeZone?: string;
      fractionalSecondDigits?: 1 | 2 | 3;
    }

    interface DateTimeFormatPart {
      type:
        | "day"
        | "dayPeriod"
        | "era"
        | "hour"
        | "literal"
        | "minute"
        | "month"
        | "second"
        | "timeZoneName"
        | "weekday"
        | "year"
        | "fractionalSecond";
      value: string;
    }

    interface ResolvedDateTimeFormatOptions {
      locale: string;
      calendar: string;
      numberingSystem: string;
      timeZone: string;
      hour12?: boolean;
      hourCycle?: "h11" | "h12" | "h23" | "h24";
      weekday?: "long" | "short" | "narrow";
      era?: "long" | "short" | "narrow";
      year?: "numeric" | "2-digit";
      month?: "numeric" | "2-digit" | "long" | "short" | "narrow";
      day?: "numeric" | "2-digit";
      hour?: "numeric" | "2-digit";
      minute?: "numeric" | "2-digit";
      second?: "numeric" | "2-digit";
      timeZoneName?: "short" | "long" | "shortOffset" | "longOffset";
      fractionalSecondDigits?: 1 | 2 | 3;
    }

    interface DateTimeFormat {
      /**
       * Format a date according to the locale and options.
       * @param date - Date to format (defaults to current time)
       */
      format(date?: Date | number): string;

      /**
       * Format a date to an array of parts.
       * @param date - Date to format (defaults to current time)
       */
      formatToParts(date?: Date | number): DateTimeFormatPart[];

      /**
       * Return the resolved options for this formatter.
       */
      resolvedOptions(): ResolvedDateTimeFormatOptions;

      readonly [Symbol.toStringTag]: "Intl.DateTimeFormat";
    }

    interface DateTimeFormatConstructor {
      new (
        locales?: string | string[],
        options?: DateTimeFormatOptions
      ): DateTimeFormat;
      (
        locales?: string | string[],
        options?: DateTimeFormatOptions
      ): DateTimeFormat;
    }

    var DateTimeFormat: DateTimeFormatConstructor;
  }

  var Intl: typeof Intl;
}
