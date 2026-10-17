<script setup lang="ts">
import { ref, PropType } from 'vue';
import { Visibility } from '@martichou/core_lib/bindings/Visibility';
import { TauriVM } from '../vue_lib/helper/ParamsHelper';
import { utils } from '../vue_lib';

const props = defineProps({
	vm: { type: Object as PropType<TauriVM>, required: true }
});
const saving = ref(false);
const error = ref('');

async function choose(visibility: Visibility) {
	if (saving.value) return;
	saving.value = true;
	error.value = '';
	try {
		await utils.setVisibility(props.vm, visibility);
	} catch {
		error.value = 'Unable to save. This computer is still hidden. Please try again.';
	} finally {
		saving.value = false;
	}
}
</script>

<template>
	<section v-if="vm.visibilityRecovered" role="alert" class="setting-row rounded-xl mx-5 mb-4 p-4">
		<p class="font-semibold">
			Choose who can find this computer
		</p>
		<p class="text-sm text-muted mt-1">
			Your saved visibility could not be read. This computer is hidden until you choose a setting.
			Your other settings have been kept.
		</p>
		<div class="flex gap-2 mt-3">
			<button type="button" class="btn btn-secondary" :disabled="saving" @click="choose('Invisible')">
				Keep hidden
			</button>
			<button type="button" class="btn btn-secondary" :disabled="saving" @click="choose('Temporarily')">
				Visible for 1 minute
			</button>
			<button type="button" class="btn btn-primary" :disabled="saving" @click="choose('Visible')">
				Visible to everyone
			</button>
		</div>
		<p v-if="error" class="text-sm mt-2" style="color: var(--rqs-danger)">
			{{ error }}
		</p>
	</section>
</template>
