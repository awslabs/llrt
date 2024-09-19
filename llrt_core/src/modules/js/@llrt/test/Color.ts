const NAMES = [
  "black",
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  "white",
  "default",
] as const;

type BgType<T extends string> = T extends `${infer A}${infer B}`
  ? `bg${Uppercase<A>}${B}`
  : T;

type BrightType<T extends string> = `${T}Bright`;

type ColorNames = {
  [k in (typeof NAMES)[number]]: number;
} & {
  [k in BgType<(typeof NAMES)[number]>]: number;
} & {
  [k in BrightType<(typeof NAMES)[number]>]: number;
} & {
  [k in BgType<BrightType<(typeof NAMES)[number]>>]: number;
};

type Options = {
  color?: number;
  bgColor?: number;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
} & Partial<ColorNames>;

class Color {
  private static CODES = (() => {
    return NAMES.reduce<Record<string, number>>(
      (acc, name: string, i: number) => {
        let code = i + 30;

        const COLOR = Color as any;

        const bgName = `bg${name[0].toUpperCase()}${name.substring(1)}`;
        const brightName = `${name}Bright`;
        const bgBrightName = `${bgName}Bright`;
        acc[name] = code;
        acc[brightName] = code + 60;
        acc[bgName] = code + 10;
        acc[bgBrightName] = code + 70;

        COLOR[name] = Color.colorizer(code);
        COLOR[brightName] = Color.colorizer(code + 60);

        COLOR[bgName] = Color.colorizer(0, {
          bgColor: code + 10,
        });
        COLOR[bgBrightName] = Color.colorizer(0, {
          bgColor: code + 70,
        });

        return acc;
      },
      {}
    ) as ColorNames;
  })();

  static RESET: string = "\x1b[0m";

  static colorizer(
    color: number,
    options: Options = {},
    option?: keyof Options
  ) {
    options.color = color;
    if (option) {
      const colorOption = Color.CODES && (Color.CODES as any)[option];
      if (colorOption) {
        if (option.startsWith("bg")) {
          options.bgColor = colorOption;
        } else {
          options.color = colorOption;
        }
      } else {
        if (!options[option]) {
          (options as any)[option] = true;
        }
      }
    }

    return new Proxy(() => {}, {
      get(target: unknown, prop: any, receiver: any) {
        return Color.colorizer(color, options, prop);
      },
      apply(target: unknown, thisArg: any, args: string[]) {
        let colorCode = `${options.color}`;
        let modes: number[] = [];

        if (options.bgColor) {
          colorCode = `${colorCode};${options.bgColor}`;
        }

        if (options.bold) {
          modes.push(1);
        }
        if (options.dim) {
          modes.push(2);
        }
        if (options.italic) {
          modes.push(3);
        }
        if (options.underline) {
          modes.push(4);
        }
        if (options.strikethrough) {
          modes.push(9);
        }

        if (modes.length == 1) {
          colorCode = `${modes[0]};${colorCode}`;
          modes = [];
        }

        return `\x1b[${colorCode}m${modes.map((m) => `\x1b[${m}m`).join("")}${args.join(
          " "
        )}${Color.RESET}`;
      },
    });
  }
}

type ColorizerReturnType = ((text: string) => string) & {
  [k in keyof (ColorNames & Required<Options>)]: ColorizerReturnType;
};

type ClassType = typeof Color & {
  [k in keyof ColorNames]: ColorizerReturnType;
};

export default Color as ClassType;
