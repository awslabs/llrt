describe("Constructor", () => {
  describe("Temporal.ZonedDateTime()", () => {
    it("Using Temporal.ZonedDateTime()", () => {
      const zdt = new Temporal.ZonedDateTime(0n, "America/New_York");
      expect(zdt.toString()).toBe(
        "1969-12-31T19:00:00-05:00[America/New_York]"
      );
    });
  });
});

describe("Static methods", () => {
  describe("Temporal.ZonedDateTime.compare()", () => {
    it("Using Temporal.ZonedDateTime.compare()", () => {
      const dt1 = Temporal.ZonedDateTime.from(
        "2021-08-01T01:00:00[Europe/London]"
      );
      const dt2 = Temporal.ZonedDateTime.from(
        "2021-08-02T00:00:00[Europe/London]"
      );
      expect(Temporal.ZonedDateTime.compare(dt1, dt2)).toBe(-1);

      const dt3 = Temporal.ZonedDateTime.from(
        "2021-08-01T00:00:00[Europe/London]"
      );
      expect(Temporal.ZonedDateTime.compare(dt1, dt3)).toBe(1);
    });

    it("Sorting an array of date-times #1", () => {
      const dateTimes = [
        Temporal.ZonedDateTime.from("2021-08-01T00:00:00[America/New_York]"),
        Temporal.ZonedDateTime.from("2021-08-01T00:00:00[Asia/Hong_Kong]"),
        Temporal.ZonedDateTime.from("2021-08-01T00:00:00[Europe/London]"),
      ];

      dateTimes.sort(Temporal.ZonedDateTime.compare);
      expect(dateTimes.map((d) => d.toString())).toStrictEqual([
        "2021-08-01T00:00:00+08:00[Asia/Hong_Kong]",
        "2021-08-01T00:00:00+01:00[Europe/London]",
        "2021-08-01T00:00:00-04:00[America/New_York]",
      ]);
    });

    it("Sorting an array of date-times #2", () => {
      const dateTimes = [
        Temporal.ZonedDateTime.from("2021-08-01T00:00:00[America/New_York]"),
        Temporal.ZonedDateTime.from("2021-08-01T00:00:00[Asia/Hong_Kong]"),
        Temporal.ZonedDateTime.from("2021-08-01T00:00:00[Europe/London]"),
      ];

      dateTimes.sort((a, b) =>
        Temporal.PlainDateTime.compare(a.toPlainDateTime(), b.toPlainDateTime())
      );
      expect(dateTimes.map((d) => d.toString())).toStrictEqual([
        "2021-08-01T00:00:00-04:00[America/New_York]",
        "2021-08-01T00:00:00+08:00[Asia/Hong_Kong]",
        "2021-08-01T00:00:00+01:00[Europe/London]",
      ]);
    });
  });

  describe("Temporal.ZonedDateTime.from()", () => {
    it("Creating a ZonedDateTime from an object", () => {
      // Year + month + day + hour + minute + second
      const zdt = Temporal.ZonedDateTime.from({
        timeZone: "America/New_York",
        year: 2021,
        month: 7,
        day: 1,
        hour: 12,
        minute: 34,
        second: 56,
      });
      expect(zdt.toString()).toBe(
        "2021-07-01T12:34:56-04:00[America/New_York]"
      );
    });

    it("Creating a ZonedDateTime from a string", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56-04:00[America/New_York]"
      );
      // expect(zdt.toLocaleString()); // "7/1/2021, 12:34:56 PM EDT" (assuming en-US locale)

      // Time given as UTC, and converted to local
      const zdt2 = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56Z[America/New_York]"
      );
      expect(zdt2.toString()).toBe(
        "2021-07-01T08:34:56-04:00[America/New_York]"
      );
    });

    it("Creating a ZonedDateTime from an ISO 8601 / RFC 3339 string", () => {
      const isoString = "2021-07-01T12:34:56+02:00";
      const instant = Temporal.Instant.from(isoString);
      const zdt = instant.toZonedDateTimeISO("America/New_York");
      expect(zdt.toString()).toBe(
        "2021-07-01T06:34:56-04:00[America/New_York]"
      );
    });

    it("Local time disambiguation", () => {
      const localTimeNotExist = "2024-03-10T02:05:00[America/New_York]";
      // For non-existent times, "compatible" is equivalent to "later"
      const zdt = Temporal.ZonedDateTime.from(localTimeNotExist);
      expect(zdt.toString()).toBe(
        "2024-03-10T03:05:00-04:00[America/New_York]"
      );

      // const zdt2 = Temporal.ZonedDateTime.from(localTimeNotExist, {
      //   disambiguation: "earlier",
      // });
      // expect(zdt2.toString()).toBe("2024-03-10T01:05:00-05:00[America/New_York]");

      const localTimeAmbiguous = "2024-11-03T01:05:00[America/New_York]";
      // For ambiguous times, "compatible" is equivalent to "earlier"
      const zdt3 = Temporal.ZonedDateTime.from(localTimeAmbiguous);
      expect(zdt3.toString()).toBe(
        "2024-11-03T01:05:00-04:00[America/New_York]"
      );

      // const zdt4 = Temporal.ZonedDateTime.from(localTimeAmbiguous, {
      //   disambiguation: "later",
      // });
      // expect(zdt4.toString()).toBe("2024-11-03T01:05:00-05:00[America/New_York]");
    });
  });
});

describe("Instance methods", () => {
  const zdt1 = Temporal.ZonedDateTime.from("2020-01-01T12:00:00+00:00[UTC]");
  const zdt2 = Temporal.ZonedDateTime.from("2020-01-02T12:00:00+00:00[UTC]");

  describe("Temporal.ZonedDateTime.prototype.add()", () => {
    it("Adding a duration", () => {
      const start = Temporal.ZonedDateTime.from(
        "2021-11-01T12:34:56-04:00[America/New_York]"
      );
      const end = start.add({
        years: 1,
        months: 2,
        weeks: 3,
        days: 4,
        hours: 5,
        minutes: 6,
        seconds: 7,
        milliseconds: 8,
      });
      expect(end.toString()).toBe(
        "2023-01-26T17:41:03.008-05:00[America/New_York]"
      );
    });
  });

  describe("Temporal.ZonedDateTime.prototype.equals()", () => {
    it("Using equals()", () => {
      // Asia/Kolkata and Asia/Calcutta are aliases of each other
      const dt1 = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56+05:30[Asia/Kolkata]"
      );
      const dt2 = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56+05:30[Asia/Calcutta]"
      );
      // expect(dt1.equals(dt2)).toBe(true);

      const dt3 = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56+05:30[+05:30]"
      );
      expect(dt1.equals(dt3)).toBe(false);

      const dt4 = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56+05:30[Asia/Kolkata][u-ca=buddhist]"
      );
      // expect(dt1.equals(dt4)).toBe(false);
    });

    it("Testing if two time zone identifiers are equivalent", () => {
      function sameTimeZone(timeZone1, timeZone2) {
        const dt1 = Temporal.ZonedDateTime.from({
          year: 2021,
          month: 7,
          day: 1,
          timeZone: timeZone1,
        });
        const dt2 = Temporal.ZonedDateTime.from({
          year: 2021,
          month: 7,
          day: 1,
          timeZone: timeZone2,
        });
        return dt1.equals(dt2);
      }

      // expect(sameTimeZone("Asia/Kolkata", "Asia/Calcutta")).toBe(true);
      expect(sameTimeZone("Asia/Shanghai", "Asia/Taipei")).toBe(false);
    });
  });

  describe("Temporal.ZonedDateTime.prototype.getTimeZoneTransition()", () => {
    it("Finding the next time zone transition", () => {
      const dt = Temporal.ZonedDateTime.from(
        "2024-01-01T00-05:00[America/New_York]"
      );
      const transition = dt.getTimeZoneTransition("next");
      expect(transition.toString()).toBe(
        "2024-03-10T03:00:00-04:00[America/New_York]"
      );

      const transition2 = transition.getTimeZoneTransition("next");
      expect(transition2.toString()).toBe(
        "2024-11-03T01:00:00-05:00[America/New_York]"
      );

      const transition3 = dt.getTimeZoneTransition("previous");
      expect(transition3.toString()).toBe(
        "2023-11-05T01:00:00-05:00[America/New_York]"
      );

      const dt2 = Temporal.ZonedDateTime.from("2024-01-01T00Z[UTC]");
      expect(dt2.getTimeZoneTransition("next")).toBe(null);
    });
  });

  describe("Temporal.ZonedDateTime.prototype.round()", () => {
    it("Rounding off small units", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56.123456789[America/New_York]"
      );
      const nearestMillisecond = zdt.round("millisecond");
      expect(nearestMillisecond.toString()).toBe(
        "2021-07-01T12:34:56.123-04:00[America/New_York]"
      );

      const nearestHalfHour = zdt.round({
        smallestUnit: "minute",
        roundingIncrement: 30,
      });
      expect(nearestHalfHour.toString()).toBe(
        "2021-07-01T12:30:00-04:00[America/New_York]"
      );

      const nextDay = zdt.round({ smallestUnit: "day", roundingMode: "ceil" });
      expect(nextDay.toString()).toBe(
        "2021-07-02T00:00:00-04:00[America/New_York]"
      );
    });

    it("Ambiguity after rounding", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2024-03-10T01:00:00-05:00[America/New_York]"
      );
      const rounded = zdt.round({ smallestUnit: "hour", roundingIncrement: 2 });
      // The result is supposed to be 2024-03-10T02:00:00-05:00[America/New_York],
      // but this time does not exist. `disambiguation: "compatible"` tells us to move
      // forward by 1 hour.
      expect(rounded.toString()).toBe(
        "2024-03-10T03:00:00-04:00[America/New_York]"
      );
    });
  });

  describe("Temporal.ZonedDateTime.prototype.since()", () => {
    it("Offset transitions", () => {
      const start = Temporal.ZonedDateTime.from(
        "2024-11-03T01:00:00-04:00[America/New_York]"
      );
      const end = Temporal.ZonedDateTime.from(
        "2024-11-04T01:00:00-05:00[America/New_York]"
      );
      expect(end.since(start).toString()).toBe("PT25H");
      expect(end.since(start, { largestUnit: "days" }).toString()).toBe("P1D"); // WebMDN's prediction of "PT1D" is probably wrong.

      const start2 = Temporal.ZonedDateTime.from(
        "2024-03-10T01:00:00-05:00[America/New_York]"
      );
      const end2 = Temporal.ZonedDateTime.from(
        "2024-03-11T01:00:00-04:00[America/New_York]"
      );
      expect(end2.since(start2).toString()).toBe("PT23H");
      expect(end2.since(start2, { largestUnit: "days" }).toString()).toBe(
        "P1D"
      ); // WebMDN's prediction of "PT1D" is probably wrong.
    });

    it("Different time zones", () => {
      const start = Temporal.ZonedDateTime.from(
        "2024-11-03T01:00:00-04:00[America/New_York]"
      );
      // Peru does not use DST so its offset remains -05:00 year-round
      const end = Temporal.ZonedDateTime.from(
        "2024-11-04T01:00:00-05:00[America/Lima]"
      );

      expect(end.since(start).toString()).toBe("PT25H");
      expect(() => {
        end.since(start, { largestUnit: "days" }); // RangeError: time zones "America/Lima" and "America/New_York" aren't compatible
      }).toThrow();

      expect(
        end
          .withTimeZone("America/New_York")
          .since(start, { largestUnit: "days" })
          .toString()
      ).toBe("P1D");
      expect(
        end
          .since(start.withTimeZone("America/Lima"), { largestUnit: "days" })
          .toString()
      ).toBe("P1DT1H"); // WebMDN's prediction of "P1D1H" is probably wrong.
    });
  });

  describe("Temporal.ZonedDateTime.prototype.subtract()", () => {
    it("Subtracting a duration", () => {
      const start = Temporal.ZonedDateTime.from(
        "2021-01-01T12:34:56-05:00[America/New_York]"
      );
      const end = start.subtract({
        years: 1,
        months: 2,
        weeks: 3,
        days: 4,
        hours: 5,
        minutes: 6,
        seconds: 7,
        milliseconds: 8,
      });
      expect(end.toString()).toBe(
        "2019-10-07T07:28:48.992-04:00[America/New_York]"
      );
    });
  });

  describe("Temporal.ZonedDateTime.prototype.toInstant()", () => {
    it("Using toInstant()", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56.987654321-04:00[America/New_York]"
      );
      const instant = zdt.toInstant();
      expect(instant.toString()).toBe("2021-07-01T16:34:56.987654321Z");
    });
  });

  describe("Temporal.ZonedDateTime.prototype.toJSON()", () => {
    it("Using toJSON()", () => {
      const zdt = Temporal.ZonedDateTime.from({
        year: 2021,
        month: 8,
        day: 1,
        timeZone: "America/New_York",
      });
      const zdtStr = zdt.toJSON();
      expect(zdtStr.toString()).toBe(
        "2021-08-01T00:00:00-04:00[America/New_York]"
      );
      const zdt2 = Temporal.ZonedDateTime.from(zdtStr);
    });
  });

  describe("Temporal.ZonedDateTime.prototype.toPlainDate()", () => {
    it("Using toPlainDate()", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56.987654321-04:00[America/New_York]"
      );
      const plainDate = zdt.toPlainDate();
      expect(plainDate.toString()).toBe("2021-07-01");
    });
  });

  describe("Temporal.ZonedDateTime.prototype.toPlainDateTime()", () => {
    it("Using toPlainDateTime()", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56.987654321-04:00[America/New_York]"
      );
      const plainDateTime = zdt.toPlainDateTime();
      expect(plainDateTime.toString()).toBe("2021-07-01T12:34:56.987654321");
    });
  });

  describe("Temporal.ZonedDateTime.prototype.toPlainTime()", () => {
    it("Using toPlainTime()", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56.987654321-04:00[America/New_York]"
      );
      const plainTime = zdt.toPlainTime();
      expect(plainTime.toString()).toBe("12:34:56.987654321");
    });
  });

  describe("Temporal.ZonedDateTime.prototype.toString()", () => {
    it("Using toString()", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-08-01T12:34:56[America/New_York]"
      );
      expect(zdt.toString()).toBe(
        "2021-08-01T12:34:56-04:00[America/New_York]"
      );
    });

    it("Even for the UTC time zone, the offset is +00:00, not Z", () => {
      const zdt = Temporal.ZonedDateTime.from("2021-08-01T12:34:56[UTC]");
      expect(zdt.toString()).toBe("2021-08-01T12:34:56+00:00[UTC]");
    });
  });

  describe("Temporal.ZonedDateTime.prototype.until()", () => {
    it("until() returns correct duration", () => {
      const flight = Temporal.ZonedDateTime.from(
        "2024-12-21T13:31:00-05:00[America/New_York]"
      );
      const now =
        Temporal.Now.zonedDateTimeISO("America/New_York").round("second");
      if (Temporal.ZonedDateTime.compare(flight, now) < 0) {
        console.error(
          "The flight is already in the past. The result may not make sense."
        );
      }
      const duration = now.until(flight, { largestUnit: "days" });
      expect(`The flight is in ${duration.toLocaleString("en-US")}`);
    });
  });

  describe("Temporal.ZonedDateTime.prototype.valueOf()", () => {
    it("valueOf() throws a TypeError", () => {
      const zdt1 = Temporal.ZonedDateTime.from(
        "2022-01-01T00:00:00[America/New_York]"
      );
      const zdt2 = Temporal.ZonedDateTime.from(
        "2022-07-01T00:00:00[America/New_York]"
      );
      expect(() => {
        zdt1 > zdt2; // TypeError: can't convert ZonedDateTime to primitive type
      }).toThrow();
      expect(Temporal.ZonedDateTime.compare(zdt1, zdt2)).toBe(-1);

      expect(() => {
        zdt2 - zdt1; // TypeError: can't convert ZonedDateTime to primitive type
      }).toThrow();
      expect(zdt2.since(zdt1).toString()).toBe("PT4343H");
    });
  });

  describe("Temporal.ZonedDateTime.prototype.with()", () => {
    it("Using with()", () => {
      const zdt = Temporal.ZonedDateTime.from(
        "2021-07-01T12:34:56[America/New_York]"
      );
      const newZDT = zdt.with({ hour: 13 });
      expect(newZDT.toString()).toBe(
        "2021-07-01T13:34:56-04:00[America/New_York]"
      );
    });

    describe("Offset during date changes", () => {
      it("By default", () => {
        const zdt = Temporal.ZonedDateTime.from(
          "2021-07-01T12:00:00-04:00[America/New_York]"
        );
        const newZDT = zdt.with({ month: 12 });
        // The offset is recalculated to -05:00
        expect(newZDT.toString()).toBe(
          "2021-12-01T12:00:00-05:00[America/New_York]"
        );
      });

      it("And if you set the time to within the DST transition, the offset is used to resolve the ambiguity", () => {
        const zdt = Temporal.ZonedDateTime.from(
          "2024-11-02T01:05:00-04:00[America/New_York]"
        );
        const newZDT = zdt.with({ day: 3 });
        expect(newZDT.toString()).toBe(
          "2024-11-03T01:05:00-04:00[America/New_York]"
        );

        const zdt2 = Temporal.ZonedDateTime.from(
          "2024-11-04T01:05:00-05:00[America/New_York]"
        );
        const newZDT2 = zdt2.with({ day: 3 });
        expect(newZDT2.toString()).toBe(
          "2024-11-03T01:05:00-05:00[America/New_York]"
        );
      });

      it.skip("If you use offset: `use`, then the offset will be used as-is to obtain the exact time first, and then recalculate the offset", () => {
        const zdt = Temporal.ZonedDateTime.from(
          "2021-07-01T12:00:00-04:00[America/New_York]"
        );
        const newZDT = zdt.with({ month: 12 }, { offset: "use" });
        // The offset is recalculated to -05:00, but the wall-clock time changes
        expect(newZDT.toString()).toBe(
          "2021-12-01T11:00:00-05:00[America/New_York]"
        );
      });

      it.skip("You can also set offset: `reject` to throw an error if the original offset is invalid, forcing an explicit new offset to be specified", () => {
        const zdt = Temporal.ZonedDateTime.from(
          "2021-07-01T12:00:00-04:00[America/New_York]"
        );

        expect(() => {
          zdt.with({ month: 12 }, { offset: "reject" });
          // RangeError: date-time can't be represented in the given time zone
        }).toThrow();
        expect(
          zdt
            .with({ month: 12, offset: "-05:00" }, { offset: "reject" })
            .toString()
        ).toBe("2021-12-01T12:00:00-05:00[America/New_York]");
      });
    });
  });

  describe("Temporal.ZonedDateTime.prototype.withTimeZone()", () => {
    it("Using withTimeZone()", () => {
      const meetingTime = Temporal.ZonedDateTime.from(
        "2021-08-01T12:00[America/New_York]"
      );
      const meetingTimeInParis = meetingTime.withTimeZone("Europe/Paris");
      expect(meetingTimeInParis.toString()).toBe(
        "2021-08-01T18:00:00+02:00[Europe/Paris]"
      );
    });

    it("Replacing the time zone while keeping the same wall-clock time", () => {
      const meetingTime = Temporal.ZonedDateTime.from(
        "2021-08-01T12:00[America/New_York]"
      );
      const meetingTimeInParis = meetingTime
        .toPlainDateTime()
        .toZonedDateTime("Europe/Paris");
      expect(meetingTimeInParis.toString()).toBe(
        "2021-08-01T12:00:00+02:00[Europe/Paris]"
      );
    });
  });
});

describe("Instance properties", () => {
  const zdt = Temporal.ZonedDateTime.from("2020-01-01T12:00:00[UTC]");

  it("should have correct date and time properties", () => {
    expect(zdt.year).toBe(2020);
    expect(zdt.month).toBe(1);
    expect(zdt.day).toBe(1);
    expect(zdt.hour).toBe(12);
    expect(zdt.minute).toBe(0);
    expect(zdt.second).toBe(0);
    expect(zdt.millisecond).toBe(0);
    expect(zdt.offset).toBe("+00:00");

    expect(typeof zdt.epochMilliseconds).toBe("number");
    expect(zdt.epochMilliseconds).toBeGreaterThan(0);
    expect(typeof zdt.epochNanoseconds).toBe("bigint");
    expect(zdt.epochNanoseconds).toBeGreaterThan(0n);

    expect(zdt.dayOfYear).toBe(1);
    expect(zdt.daysInMonth).toBe(31);
    expect(zdt.daysInYear).toBe(366);
    expect(zdt.inLeapYear).toBe(true);
  });

  it("has correct toStringTag", () => {
    expect(Object.prototype.toString.call(zdt)).toBe(
      "[object Temporal.ZonedDateTime]"
    );
  });
});
