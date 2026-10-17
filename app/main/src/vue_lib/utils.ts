import { Visibility } from '@martichou/core_lib/bindings/Visibility';
import { TauriVM } from './helper/ParamsHelper';
import { autostartKey, DisplayedItem, downloadPathKey, realcloseKey, startminimizedKey, stateToDisplay } from './types';
import { SendInfo } from '@martichou/core_lib/bindings/SendInfo';
import { ChannelMessage } from '@martichou/core_lib/bindings/ChannelMessage';
import { ChannelAction } from '@martichou/core_lib';
import { State } from '@martichou/core_lib/bindings/State';
import { gt } from 'semver';

const terminalHistoryStates = new Set<State>([
	'Finished',
	'Cancelled',
	'Rejected',
	'Disconnected',
]);

function isTerminalHistoryState(state: State | null | undefined): boolean {
	return state !== null && state !== undefined && terminalHistoryStates.has(state);
}

function hasLiveDisplayContent(vm: TauriVM): boolean {
	if (vm.outboundPayload !== undefined || vm.endpointsInfo.length > 0) {
		return true;
	}

	return vm.requests.some((request) => {
		const state = request.state ?? 'Initial';
		return stateToDisplay.includes(state) && !isTerminalHistoryState(state);
	});
}

function _displayedItems(vm: TauriVM): Array<DisplayedItem> {
	const ndisplayed = new Array<DisplayedItem>();

	vm.endpointsInfo.forEach((el) => {
		const idx = ndisplayed.findIndex((nel) => el.id == nel.id);
		if (idx !== -1) return;

		ndisplayed.push({
			id: el.id,
			name: el.name ?? 'Unknown',
			deviceType: el.rtype ?? 'Unknown',
			endpoint: true,
		})
	});

	vm.requests.filter((el) => stateToDisplay.includes(el.state ?? 'Initial')).forEach((el) => {
		const idx = ndisplayed.findIndex((nel) => el.id == nel.id);
		const existing = idx !== -1 ? ndisplayed[idx] : undefined;

		if (
			vm.outboundPayload !== undefined
			&& existing?.endpoint
			&& isTerminalHistoryState(el.state)
		) {
			return;
		}

		const elem: DisplayedItem = {
			id: el.id,
			name: el.meta?.source?.name ?? existing?.name ?? 'Unknown',
			deviceType: el.meta?.source?.device_type ?? existing?.deviceType ?? 'Unknown',
			endpoint: false,

			state: el.state ?? undefined,
			pin_code: el.meta?.pin_code ?? undefined,
			destination: el.meta?.destination ?? undefined,
			files: el.meta?.files ?? undefined,
			text_description: el.meta?.text_description ?? undefined,
			text_payload: el.meta?.text_payload ?? undefined,
			text_type: el.meta?.text_type ?? undefined,
			ack_bytes: (el.meta?.ack_bytes as number | undefined) ?? undefined,
			total_bytes: (el.meta?.total_bytes as number | undefined) ?? undefined,
			error: el.error ?? undefined,
		};

		if (idx !== -1) {
			ndisplayed.splice(idx, 1, elem);
		} else {
			ndisplayed.push(elem)
		}
	});

	return ndisplayed;
}

async function setAutoStart(vm: TauriVM, autostart: boolean) {
	if (autostart) {
		await vm.enable();
	} else {
		await vm.disable();
	}

	await vm.store.set(autostartKey, autostart);
	await vm.store.save();
	vm.autostart = autostart;
}

async function applyAutoStart(vm: TauriVM) {
	vm.autostart = await vm.store.get(autostartKey) ?? false;

	if (vm.autostart) {
		await vm.enable();
	} else {
		await vm.disable();
	}
}

async function setRealClose(vm: TauriVM, realclose: boolean) {
	await vm.store.set(realcloseKey, realclose);
	await vm.store.save();
	vm.realclose = realclose;
}

async function getRealclose(vm: TauriVM) {
	vm.realclose = await vm.store.get(realcloseKey) ?? false;
}

async function setStartMinimized(vm: TauriVM, startminimized: boolean) {
	await vm.store.set(startminimizedKey, startminimized);
	await vm.store.save();
	vm.startminimized = startminimized;
}

async function getStartMinimized(vm: TauriVM) {
	vm.startminimized = await vm.store.get(startminimizedKey) ?? false;
}

async function setVisibility(vm: TauriVM, visibility: Visibility) {
	await vm.invoke('change_visibility', { message: visibility });
	vm.visibility = visibility;
	vm.visibilityRecovered = false;
}

async function getVisibility(vm: TauriVM) {
	const [visibility, recovered] = await vm.invoke('get_receiving_status') as [Visibility, boolean];
	vm.visibility = visibility;
	vm.visibilityRecovered = recovered;
}

async function invertVisibility(vm: TauriVM) {
	if (vm.visibility === 'Temporarily') {
		return;
	}

	if (vm.visibility === 'Visible') {
		return await vm.setVisibility(vm, 'Invisible');
	}

	return await vm.setVisibility(vm, 'Visible');
}

async function clearSending(vm: TauriVM) {
	const payload = vm.outboundPayload;
	if (payload && 'EphemeralFiles' in payload) {
		for (const path of payload.EphemeralFiles) {
			try {
				await vm.invoke('remove_ephemeral_file', { path });
			} catch (error) {
				console.warn('Could not remove ephemeral clipboard file', error);
			}
		}
	}

	await vm.invoke('stop_discovery');
	vm.outboundPayload = undefined;
	vm.discoveryRunning = false;
	vm.endpointsInfo = [];
}

function removeRequest(vm: TauriVM, id: string) {
	const idx = vm.requests.findIndex((el) => el.id === id);

	if (idx !== -1) {
		vm.requests.splice(idx, 1);
	}
}

async function sendInfo(vm: TauriVM, eid: string) {
	if (vm.outboundPayload === undefined) return;

	const ei = vm.endpointsInfo.find((el) => el.id === eid);
	if (!ei || !ei.ip || !ei.port) return;

	const msg: SendInfo = {
		id: ei.id,
		name: ei.name ?? 'Unknown',
		addr: ei.ip + ":" + ei.port,
		ob: vm.outboundPayload,
	};

	await vm.invoke('send_payload', { message: msg });
}

async function sendCmd(vm: TauriVM, id: string, action: ChannelAction) {
	const cm: ChannelMessage = {
		id: id,
		direction: 'FrontToLib',
		action: action,
		meta: null,
		state: null,
		rtype: null,
		error: null,
	};
	console.log("js2rs:", cm);

	await vm.invoke('send_to_rs', { message: cm });
}

function blured() {
	(document.activeElement as any).blur();
}

function getProgress(item: DisplayedItem): string {
	const value = item.ack_bytes! / item.total_bytes! * 100;
	return `--progress: ${value}`;
}

async function setDownloadPath(vm: TauriVM, dest: string) {
	await vm.invoke('change_download_path', { message: dest });
	await vm.store.set(downloadPathKey, dest);
	await vm.store.save();
	vm.downloadPath = dest;
}

async function getDownloadPath(vm: TauriVM) {
	vm.downloadPath = await vm.store.get(downloadPathKey) ?? undefined;
}

async function getLatestVersion(vm: TauriVM) {
	try {
		const response = await fetch('https://api.github.com/repos/morozhenko-apps/rquickshare/releases/latest');
		if (!response.ok) {
			throw new Error(`Error: ${response.status} ${response.statusText}`);
		}
		const data = await response.json();
		const new_version = data.tag_name.substring(1);
		console.log(`Latest version: ${vm.version} vs ${new_version}`);

		if (vm.version && gt(new_version, vm.version)) {
			vm.new_version = new_version;
		}
	} catch (err) {
		console.error(err);
	}
}

// Default export
export const utils = {
	_displayedItems,
	hasLiveDisplayContent,
	setAutoStart,
	applyAutoStart,
	setRealClose,
	getRealclose,
	setVisibility,
	getVisibility,
	invertVisibility,
	clearSending,
	removeRequest,
	sendInfo,
	sendCmd,
	blured,
	getProgress,
	setDownloadPath,
	getDownloadPath,
	getLatestVersion,
	setStartMinimized,
	getStartMinimized
};
export type UtilsType = typeof utils;
