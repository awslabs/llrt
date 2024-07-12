/**
 * The `path` module provides utilities for working with file and directory
 * paths. It can be accessed using:
 *
 * ```js
 * import path from 'path';
 * ```
 */
declare module "path" {
  namespace path {
    /**
     * A parsed path object generated by path.parse() or consumed by path.format().
     */
    interface ParsedPath {
      /**
       * The root of the path such as '/' or 'c:\'
       */
      root: string;
      /**
       * The full directory path such as '/home/user/dir' or 'c:\path\dir'
       */
      dir: string;
      /**
       * The file name including extension (if any) such as 'index.html'
       */
      base: string;
      /**
       * The file extension (if any) such as '.html'
       */
      ext: string;
      /**
       * The file name without extension (if any) such as 'index'
       */
      name: string;
    }
    interface FormatInputPathObject {
      /**
       * The root of the path such as '/' or 'c:\'
       */
      root?: string | undefined;
      /**
       * The full directory path such as '/home/user/dir' or 'c:\path\dir'
       */
      dir?: string | undefined;
      /**
       * The file name including extension (if any) such as 'index.html'
       */
      base?: string | undefined;
      /**
       * The file extension (if any) such as '.html'
       */
      ext?: string | undefined;
      /**
       * The file name without extension (if any) such as 'index'
       */
      name?: string | undefined;
    }
    interface PlatformPath {
      /**
       * Normalize a string path, reducing '..' and '.' parts.
       * When multiple slashes are found, they're replaced by a single one; when the path contains a trailing slash, it is preserved. On Windows backslashes are used.
       *
       * @param path string path to normalize.
       * @throws {TypeError} if `path` is not a string.
       */
      normalize(path: string): string;
      /**
       * Join all arguments together and normalize the resulting path.
       *
       * @param paths paths to join.
       * @throws {TypeError} if any of the path segments is not a string.
       */
      join(...paths: string[]): string;
      /**
       * The right-most parameter is considered {to}. Other parameters are considered an array of {from}.
       *
       * Starting from leftmost {from} parameter, resolves {to} to an absolute path.
       *
       * If {to} isn't already absolute, {from} arguments are prepended in right to left order,
       * until an absolute path is found. If after using all {from} paths still no absolute path is found,
       * the current working directory is used as well. The resulting path is normalized,
       * and trailing slashes are removed unless the path gets resolved to the root directory.
       *
       * @param paths A sequence of paths or path segments.
       * @throws {TypeError} if any of the arguments is not a string.
       */
      resolve(...paths: string[]): string;
      /**
       * Determines whether {path} is an absolute path. An absolute path will always resolve to the same location, regardless of the working directory.
       *
       * If the given {path} is a zero-length string, `false` will be returned.
       *
       * @param path path to test.
       */
      isAbsolute(path: string): boolean;
      /**
       * Return the directory name of a path. Similar to the Unix dirname command.
       *
       * @param path the path to evaluate.
       */
      dirname(path: string): string;
      /**
       * Return the last portion of a path. Similar to the Unix basename command.
       * Often used to extract the file name from a fully qualified path.
       *
       * @param path the path to evaluate.
       * @param suffix optionally, an extension to remove from the result.
       */
      basename(path: string, suffix?: string): string;
      /**
       * Return the extension of the path, from the last '.' to end of string in the last portion of the path.
       * If there is no '.' in the last portion of the path or the first character of it is '.', then it returns an empty string.
       *
       * @param path the path to evaluate.
       */
      extname(path: string): string;
      /**
       * The platform-specific file delimiter. ';' or ':'.
       */
      readonly delimiter: ";" | ":";
      /**
       * Returns an object from a path string - the opposite of format().
       *
       * @param path path to evaluate.
       * @throws {TypeError} if `path` is not a string.
       */
      parse(path: string): ParsedPath;
      /**
       * Returns a path string from an object - the opposite of parse().
       *
       * @param pathObject path to evaluate.
       */
      format(pathObject: FormatInputPathObject): string;
    }
  }
  const path: path.PlatformPath;
  export = path;
}
