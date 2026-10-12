// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils';
import { reactive } from 'vue';
import { describe, expect, test, vi } from 'vitest';
import VisibilityRecovery from '../../src/composables/VisibilityRecovery.vue';
import { utils } from '../../src/vue_lib';
import type { TauriVM } from '../../src/vue_lib/helper/ParamsHelper';

function model(): TauriVM {
	return reactive({ visibility: 'Invisible', visibilityRecovered: true, invoke: vi.fn(async () => undefined) }) as unknown as TauriVM;
}

describe('Visibility recovery', () => {
	test.each([
		['Keep hidden', 'Invisible'], ['Visible for 1 minute', 'Temporarily'], ['Visible to everyone', 'Visible']
	])('Given recovered settings When %s is chosen Then explicit save clears the persistent warning', async (label, visibility) => {
		// Arrange
		const vm = model();
		const wrapper = mount(VisibilityRecovery, { props: { vm } });
		expect(wrapper.get('[role="alert"]').text()).toContain('This computer is hidden');
		expect(vm.invoke).not.toHaveBeenCalled();
		// Act
		await wrapper.findAll('button').find(button => button.text() === label)!.trigger('click');
		await flushPromises();
		// Assert
		expect(vm.invoke).toHaveBeenCalledExactlyOnceWith('change_visibility', { message: visibility });
		expect(vm.visibility).toBe(visibility);
		expect(vm.visibilityRecovered).toBe(false);
		expect(wrapper.find('[role="alert"]').exists()).toBe(false);
	});

	test('Given a failed save When visibility is chosen Then hidden state and repair controls remain', async () => {
		// Arrange
		const vm = model();
		vi.mocked(vm.invoke).mockRejectedValueOnce(new Error('storage unavailable'));
		const wrapper = mount(VisibilityRecovery, { props: { vm } });
		// Act
		await wrapper.findAll('button')[2].trigger('click');
		await flushPromises();
		// Assert
		expect(vm.visibility).toBe('Invisible');
		expect(vm.visibilityRecovered).toBe(true);
		expect(wrapper.get('[role="alert"]').text()).toContain('Unable to save');
		expect(wrapper.findAll('button').every(button => !button.attributes('disabled'))).toBe(true);
	});

	test('Given backend recovery status When loaded Then the UI uses effective visibility without writing settings', async () => {
		// Arrange
		const vm = model();
		vi.mocked(vm.invoke).mockResolvedValueOnce(['Invisible', true]);
		// Act
		await utils.getVisibility(vm);
		// Assert
		expect(vm.invoke).toHaveBeenCalledExactlyOnceWith('get_receiving_status');
		expect(vm.visibility).toBe('Invisible');
		expect(vm.visibilityRecovered).toBe(true);
	});
});
