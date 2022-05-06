import * as fs from 'fs';
export interface IPerformanceTest {
    open_as: 'text' | 'dlt' | 'pcap';
    ignore: boolean;
    alias: string;
    expectation_ms: number;
    file: string;
}
export interface IRegularTests {
    execute_only: number[];
    list: { [key: string]: string };
    files: { [key: string]: string };
}
export interface IConfiguration {
    log_level: number;
    tests: {
        observe: {
            regular: IRegularTests;
            performance: {
                run: boolean;
                tests: { [key: string]: IPerformanceTest };
            };
        };
        search: {
            regular: IRegularTests;
        };
        cancel: {
            regular: IRegularTests;
        };
    };
}

export function readConfigurationFile(): Config {
    const config = (() => {
        const filename = (process.env as any)['JASMIN_TEST_CONFIGURATION'];
        if (typeof filename !== 'string' || filename.trim() === '') {
            return new Error(
                `To run test you should define a path to configuration file with JASMIN_TEST_CONFIGURATION=path_to_config_json_file`,
            );
        }
        if (!fs.existsSync(filename)) {
            return new Error(`Configuration file ${filename} doesn't exist`);
        }
        const buffer = fs.readFileSync(filename);
        try {
            return new Config(JSON.parse(buffer.toString().replace(/\/\*.*\*\//gi, '')));
        } catch (err) {
            return new Error(
                `Fail to parse configuration file ${filename}; error: ${
                    err instanceof Error ? err.message : err
                }`,
            );
        }
    })();
    if (config instanceof Error) {
        console.warn(`\n`);
        console.warn(`=`.repeat(81));
        console.warn(`**** ERROR ${'*'.repeat(68)}`);
        console.warn(`=`.repeat(81));
        console.warn(`Fail to read configuration file due error: ${config.message}`);
        console.warn(
            `Test will be done in the scope of tasks, which do not require configuration.`,
        );
        console.warn(`=`.repeat(81));
        console.warn(`\n`);
        process.exit(1);
    } else {
        return config;
    }
}
export class Config {
    private readonly _config: IConfiguration;

    constructor(config: IConfiguration) {
        this._config = config;
    }

    public get(): IConfiguration {
        return this._config;
    }
}