<template>
	<div class="app-shell flex flex-col w-full h-full max-w-full max-h-full overflow-hidden">
		<ToastNotification />
		<SettingsModal :vm="vm" @close="settingsOpen = false" />

		<Heading :vm="vm" :open-url="openUrl" @open-settings="settingsOpen = true" />
		<VisibilityRecovery :vm="vm" />

		<div class="flex-1 flex flex-row gap-4 px-5 pb-5 min-h-0">
			<SideMenu :vm="vm" @invert-visibility="invertVisibility(vm)" @clear-sending="clearSending(vm)" />

			<main class="app-content flex-1 flex flex-col w-full max-w-full min-w-0 min-h-0 rounded-3xl p-8 overflow-y-auto">
				<ContentStatus
					:vm="vm"
					@outbound-payload="(el: OutboundPayload) => outboundPayload = el"
					@discovery-running="discoveryRunning = true;" />

				<div
					v-for="item in displayedItems"
					:key="item.id"
					class="device-card w-full rounded-2xl flex flex-row gap-5 p-5 mb-3"
					:data-clickable="Boolean(item.endpoint)"
					:class="{'cursor-pointer': item.endpoint}"
					@click="item.endpoint && sendInfo(vm, item.id)">
					<ItemSide :item="item" />

					<div class="flex-1 flex flex-col text-sm min-w-0" :class="{'justify-center': item.state === undefined}">
						<div class="flex items-center gap-2">
							<h3 class="text-base font-semibold truncate">
								{{ item.name }}
							</h3>
							<span v-if="item.state" class="status-chip">
								{{ formatTransferState(item.state) }}
							</span>
						</div>

						<div v-if="item.state === 'WaitingForUserConsent'" class="flex-1 flex flex-col justify-between">
							<p class="mt-3 text-muted">
								Wants to share {{ item.files?.join(', ') ?? item.text_description ?? 'some file(s).' }}
							</p>
							<div class="flex flex-row justify-end gap-2 mt-3">
								<button type="button" @click.stop="sendCmd(vm, item.id, 'RejectTransfer')" class="btn btn-secondary">
									Decline
								</button>
								<button type="button" @click.stop="sendCmd(vm, item.id, 'AcceptTransfer')" class="btn btn-primary">
									Accept
								</button>
							</div>
						</div>

						<div v-else-if="['SentIntroduction', 'SendingFiles', 'ReceivingFiles'].includes(item.state ?? 'Initial')">
							<p class="mt-2 text-muted" v-if="['SentIntroduction', 'SendingFiles'].includes(item.state ?? 'Initial')">
								Sending…
							</p>
							<p class="mt-2 text-muted" v-else>
								Receiving…
							</p>
							<p v-for="f in item.files ?? []" :key="f" class="overflow-hidden whitespace-nowrap text-ellipsis mt-1">
								{{ f }}
							</p>
							<div class="flex flex-row justify-end gap-2 mt-3">
								<button type="button" @click.stop="sendCmd(vm, item.id, 'CancelTransfer')" class="btn btn-secondary">
									Cancel
								</button>
							</div>
						</div>

						<div v-else-if="item.state === 'Finished'">
							<p class="mt-2 text-muted">
								Transfer completed<span v-if="item.text_type"> · text received</span>
							</p>

							<p v-for="f in item.files ?? []" :key="f" class="overflow-hidden whitespace-nowrap text-ellipsis mt-1">
								{{ f }}
							</p>
							<p v-if="item.files" class="mt-2 overflow-hidden whitespace-nowrap text-ellipsis text-muted">
								Saved to {{ item.destination }}
							</p>

							<p v-if="item.text_type" class="!select-text cursor-text overflow-hidden whitespace-nowrap text-ellipsis mt-2">
								{{ item.text_payload }}
							</p>

							<div class="flex flex-row justify-end gap-2 mt-3">
								<button
									v-if="item.destination"
									type="button"
									@click.stop="revealDownloadItem(item.files?.[0])"
									class="btn btn-secondary">
									Open
								</button>
								<button
									v-else-if="item.text_type === 'Url' && item.text_payload"
									type="button"
									@click.stop="openUrl(item.text_payload)"
									class="btn btn-secondary">
									Open
								</button>
								<button
									v-if="item.text_type && item.text_payload"
									type="button"
									@click.stop="writeToClipboard(item.text_payload)"
									class="btn btn-secondary">
									Copy
								</button>
								<button type="button" @click.stop="removeRequest(vm, item.id)" class="btn btn-secondary">
									Clear
								</button>
							</div>
						</div>

						<div v-else-if="item.state === 'Cancelled'">
							<p class="mt-2 text-muted">
								Transfer cancelled.
							</p>
							<div class="flex flex-row justify-end mt-3">
								<button type="button" @click.stop="removeRequest(vm, item.id)" class="btn btn-secondary">
									Clear
								</button>
							</div>
						</div>

						<div v-else-if="item.state === 'Rejected'">
							<p class="mt-2 text-muted">
								Transfer rejected.
							</p>
							<div class="flex flex-row justify-end mt-3">
								<button type="button" @click.stop="removeRequest(vm, item.id)" class="btn btn-secondary">
									Clear
								</button>
							</div>
						</div>

						<div v-else-if="item.state === 'Disconnected'">
							<p class="mt-2" style="color: var(--rqs-danger)">
								{{ item.error ?? 'Unexpected disconnection.' }}
							</p>
							<div class="flex flex-row justify-end mt-3">
								<button type="button" @click.stop="removeRequest(vm, item.id)" class="btn btn-danger">
									Clear
								</button>
							</div>
						</div>
					</div>
				</div>
			</main>
		</div>
	</div>
</template>

<script lang="ts">
import { ref, nextTick } from 'vue'
import { UnlistenFn, listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getStore } from "@tauri-apps/plugin-store";
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import { disable, enable } from '@tauri-apps/plugin-autostart';
import { open as tauriDialog } from '@tauri-apps/plugin-dialog';
import { open } from '@tauri-apps/plugin-shell';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

import { ChannelMessage } from '@martichou/core_lib/bindings/ChannelMessage';
import { EndpointInfo } from '@martichou/core_lib/bindings/EndpointInfo';
import { OutboundPayload } from '@martichou/core_lib/bindings/OutboundPayload';
import { Visibility } from '@martichou/core_lib/bindings/Visibility';

import { ToastNotification, ToDelete, stateToDisplay, formatTransferState, autostartKey, DisplayedItem, useToastStore, opt, ToastType, utils, writeClipboardWithFallback } from '../vue_lib';

import SettingsModal from '../composables/SettingsModal.vue';
import Heading from '../composables/Heading.vue';
import SideMenu from '../composables/SideMenu.vue';
import ContentStatus from '../composables/ContentStatus.vue';
import ItemSide from '../composables/ItemSide.vue';
import VisibilityRecovery from '../composables/VisibilityRecovery.vue';

export default {
	name: "HomePage",

	components: {
		ToastNotification,
		SettingsModal,
		Heading,
		SideMenu,
		ContentStatus,
		ItemSide,
		VisibilityRecovery
	},

	async setup() {
		const store = (await getStore(".settings.json"))!;
		const toastStore = useToastStore();

		const dialogOpen = tauriDialog;

		return {
			stateToDisplay,
			formatTransferState,
			store,
			toastStore,
			invoke,
			getVersion,
			enable,
			disable,
			dialogOpen,
			...utils
		};
	},

	data() {
		return {
			isAppInForeground: false,
			discoveryRunning: ref(false),
			isDragHovering: ref(false),

			requests: ref<ChannelMessage[]>([]),
			endpointsInfo: ref<EndpointInfo[]>([]),
			toDelete: ref<ToDelete[]>([]),
			outboundPayload: ref<OutboundPayload | undefined>(),

			// eslint-disable-next-line no-undef
			cleanupInterval: opt<NodeJS.Timeout>(),
			unlisten: Array<UnlistenFn>(),

			version: opt<string>(),

			autostart: ref<boolean>(true),
			realclose: ref<boolean>(false),
			startminimized: ref<boolean>(false),
			visibility: ref<Visibility>('Invisible'),
			visibilityRecovered: ref(false),
			downloadPath: ref<string | undefined>(),

			hostname: ref<string>(),

			settingsOpen: ref<boolean>(false),

			new_version: opt<string>(),
		};
	},

	mounted: function () {
		nextTick(async () => {
			this.hostname = await invoke('get_hostname');
			this.version = await getVersion();

			await this.getVisibility(this);

			if (!await this.store.has(autostartKey)) {
				if (!this.visibilityRecovered) await this.setAutoStart(this, false);
				else this.autostart = false;
			} else {
				await this.applyAutoStart(this);
			}

			await this.getRealclose(this);
			await this.getStartMinimized(this);
			await this.getDownloadPath(this);

			// Check permission for notification
			let permissionGranted = await isPermissionGranted();
			if (!permissionGranted) {
				const permission = await requestPermission();
				permissionGranted = permission === 'granted';
			}

			this.unlisten.push(
				await listen('rs2js_channelmessage', async (event) => {
					const cm = event.payload as ChannelMessage;
					const idx = this.requests.findIndex((el) => el.id === cm.id);

					if (cm.state === "Disconnected") {
						this.toDelete.push({
							id: cm.id,
							triggered: new Date().getTime()
						});
					}

					// TODO - Automatically open || copy to clipboard + toast

					if (idx !== -1) {
						const prev = this.requests.at(idx);
						// Update the existing message at index 'idx'
						this.requests.splice(idx, 1, {
							...cm,
							state: cm.state ?? prev!.state,
							meta: cm.meta ?? prev!.meta,
						});
					} else {
						// Push the new message if not found
						this.requests.push(cm);
					}

					return;
				})
			);

			this.unlisten.push(
				await listen('rs2js_endpointinfo', (event) => {
					const ei = event.payload as EndpointInfo;
					const idx = this.endpointsInfo.findIndex((el) => el.id === ei.id);

					if (!ei.present) {
						if (idx !== -1) {
							this.endpointsInfo.splice(idx, 1);
						}

						return;
					}

					if (idx !== -1) {
						this.endpointsInfo.splice(idx, 1, ei);
					} else {
						this.endpointsInfo.push(ei);
					}
				})
			);

			this.unlisten.push(
				await listen('visibility_updated', async () => {
					console.log("Visibility changed");
					await this.getVisibility(this);
				})
			);

			this.unlisten.push(
				await getCurrentWindow().onDragDropEvent(async (event) => {
					if (event.payload.type === 'over') {
						this.isDragHovering = true;
					} else if (event.payload.type === 'drop') {
						console.log("Dropped");
						this.isDragHovering = false;
						this.outboundPayload = {
							Files: event.payload.paths
						} as OutboundPayload;
						if (!this.discoveryRunning) await invoke('start_discovery');
						this.discoveryRunning = true;
					} else {
						this.isDragHovering = false;
					}
				})
			);

			await this.getLatestVersion(this);
		});
	},

	unmounted: function() {
		this.unlisten.forEach((el) => el());

		if (this.cleanupInterval && this.cleanupInterval[Symbol.dispose]) {
			this.cleanupInterval[Symbol.dispose]();
		}
	},

	computed: {
		vm() {
			return this;
		},
		displayedIsEmpty(): boolean {
			return !this.hasLiveDisplayContent(this)
		},
		displayedItems(): Array<DisplayedItem> {
			return this._displayedItems(this);
		}
	},

	methods: {
		writeToClipboard: async function(text: string) {
			const browserWriter = typeof navigator !== 'undefined' && navigator.clipboard?.writeText
				? (value: string) => navigator.clipboard.writeText(value)
				: undefined;
			const method = await writeClipboardWithFallback(text, writeText, browserWriter);

			if (method !== null) {
				this.toastStore.addToast("Copied to clipboard", ToastType.Success);
				return;
			}

			this.toastStore.addToast("Unable to copy text", ToastType.Error);
			console.error("Clipboard copy failed through both Tauri and browser APIs");
		},
		revealDownloadItem: async function(fileName?: string) {
			try {
				if (!fileName) {
					await invoke('open_download_destination');
					return;
				}
				await invoke('reveal_download_item', { fileName });
			} catch (e) {
				this.toastStore.addToast("Unable to show the received file", ToastType.Error);
				console.error("Error revealing received file", e);
			}
		},
		openUrl: async function(url: string) {
			try {
				await open(url);
			} catch (e) {
				this.toastStore.addToast("Error opening URL, it may not be a valid URI", ToastType.Error);
				console.error("Error opening URL", e);
			}
		},
	},
}
</script>
