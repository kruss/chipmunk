import { Observable, Subject, Subscription } from 'rxjs';
import { IAdbDevice, IAdbProcess, IAdbSession } from '../../../../../../../../common/interfaces/interface.adb';

import ElectronIpcService, { IPCMessages } from '../../../../services/service.electron.ipc';

import * as Toolkit from 'chipmunk.client.toolkit';

export interface IAmount {
    session: string;
    amount: string;
}

export enum EAdbStatus {
    init = 'init',
    ready = 'ready',
    error = 'error',
}

export class SidebarAppAdbService {

    private _status: EAdbStatus = EAdbStatus.init;
    private _subscriptions: { [key: string]: Toolkit.Subscription | Subscription } = {};
    private _subjects: {
        onAmount: Subject<IAmount>,
    } = {
        onAmount: new Subject<IAmount>(),
    };

    constructor() {
        this._subscriptions.AdbStreamUpdated = ElectronIpcService.subscribe(IPCMessages.AdbStreamUpdated, this._onAdbStreamUpdated.bind(this));
    }

    public destroy() {
        Object.keys(this._subscriptions).forEach((key: string) => {
            this._subscriptions[key].unsubscribe();
        });
    }

    public getObservable(): {
        onAmount: Observable<IAmount>,
    } {
        return {
            onAmount: this._subjects.onAmount.asObservable(),
        };
    }

    public set status(status: EAdbStatus) {
        this._status = status;
    }

    public get status(): EAdbStatus {
        return this._status;
    }

    public getDevices(request: IPCMessages.IAdbDevicesRequest): Promise<IAdbDevice[]> {
        return new Promise((resolve, reject) => {
            ElectronIpcService.request(new IPCMessages.AdbDevicesRequest(request), IPCMessages.AdbDevicesResponse).then((response: IPCMessages.AdbDevicesResponse) => {
                if (response.error !== undefined) {
                    this._status = EAdbStatus.error;
                    return reject(response.error);
                }
                this._status = EAdbStatus.ready;
                resolve(response.devices);
            }).catch((error: Error) => {
                this._status = EAdbStatus.error;
                reject(error);
            });
        });
    }

    public getProcesses(request: IPCMessages.IAdbProcessesRequest): Promise<IAdbProcess[]> {
        return new Promise((resolve, reject) => {
            ElectronIpcService.request(new IPCMessages.AdbProcessesRequest(request), IPCMessages.AdbProcessesResponse).then((response: IPCMessages.AdbProcessesResponse) => {
                if (response.error !== undefined) {
                    return reject(response.error);
                }
                resolve(response.processes);
            }).catch((error: Error) => {
                reject(error.message);
            });
        });
    }

    public start(request: IPCMessages.IAdbStartRequest): Promise<void> {
        return new Promise((resolve, reject) => {
            ElectronIpcService.request(new IPCMessages.AdbStartRequest(request), IPCMessages.AdbStartResponse).then((response: IPCMessages.AdbStartResponse) => {
                resolve();
            }).catch((error: Error) => {
                reject(error.message);
            });
        });
    }

    public stop(request: IPCMessages.IAdbStopRequest): Promise<void> {
        return new Promise((resolve, reject) => {
            ElectronIpcService.request(new IPCMessages.AdbStopRequest(request), IPCMessages.AdbStopResponse).then((response: IPCMessages.AdbStopResponse) => {
                if (response.error !== undefined) {
                    return reject(response.error);
                }
                resolve();
            }).catch((error: Error) => {
                reject(error);
            });
        });
    }

    public change(request: IPCMessages.IAdbStartRequest): Promise<void> {
        return new Promise((resolve, reject) => {
            this.start(request).then(() => {
                resolve();
            }).catch((error: string) => {
                reject(error);
            });
        });
    }

    public restore(request: IPCMessages.IAdbLoadRequest): Promise<IPCMessages.AdbLoadResponse> {
        return new Promise((resolve, reject) => {
            ElectronIpcService.request(new IPCMessages.AdbLoadRequest(request), IPCMessages.AdbLoadResponse).then((response: IPCMessages.AdbLoadResponse) => {
                resolve(response);
            }).catch((error: Error) => {
                reject(error.message);
            });
        });
    }

    public bytesToString(amount: number): string {
        if (amount < 1024) {
            return `${amount} bytes`;
        } else if (amount / 1024 < 1024) {
            return `${(amount / 1024).toFixed(2)} kB`;
        } else if (amount / 1024 / 1024 < 1024 * 1024) {
            return`${(amount / 1024 / 1024).toFixed(4)} Mb`;
        } else {
            return `${(amount / 1024 / 1024 / 1024).toFixed(5)} Gb`;
        }
    }

    private _onAdbStreamUpdated(response: IPCMessages.AdbStreamUpdated) {
        this._subjects.onAmount.next({ session: response.guid, amount: this.bytesToString(response.amount) })
    }

}