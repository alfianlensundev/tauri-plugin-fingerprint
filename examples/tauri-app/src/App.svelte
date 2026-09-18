<script>
	import { onMount } from 'svelte'
	import {
		capture,
		checkDevice,
		enroll,
		getDeviceInfo,
		identify,
		verify,
	} from 'tauri-plugin-fingerprint'

	const fingers = [
		{ value: 'right-thumb', label: 'Right thumb' },
		{ value: 'right-index', label: 'Right index' },
		{ value: 'right-middle', label: 'Right middle' },
		{ value: 'right-ring', label: 'Right ring' },
		{ value: 'right-little', label: 'Right little' },
		{ value: 'left-thumb', label: 'Left thumb' },
		{ value: 'left-index', label: 'Left index' },
		{ value: 'left-middle', label: 'Left middle' },
		{ value: 'left-ring', label: 'Left ring' },
		{ value: 'left-little', label: 'Left little' },
	]

	let device = $state(null)
	let deviceInfo = $state(null)
	let enrollment = $state(null)
	let verification = $state(null)
	let identification = $state(null)
	let captureResult = $state(null)
	let operation = $state('')
	let successMessage = $state('')
	let errorMessage = $state('')
	let enrollmentErrors = $state({})
	let verifyErrors = $state({})
	let enrollmentForm = $state({
		username: '',
		finger: 'right-index',
		samples: 4,
		timeoutSecs: 60,
		saveLocally: true,
	})
	let verifyForm = $state({
		source: 'local',
		username: '',
		threshold: 40,
		timeoutSecs: 60,
	})

	onMount(() => {
		void refreshDevice()
	})

	function formatError(error) {
		if (typeof error === 'string') {
			return error
		}

		if (error instanceof Error) {
			return error.message
		}

		try {
			return JSON.stringify(error)
		} catch {
			return 'An unknown error occurred'
		}
	}

	function formatJson(value) {
		return JSON.stringify(value, null, 2)
	}

	function sensitivityLabel(value) {
		if (value < 30) {
			return 'Permissive'
		}

		if (value < 55) {
			return 'Balanced'
		}

		return 'Strict'
	}

	async function runOperation(name, callback) {
		operation = name
		errorMessage = ''
		successMessage = ''

		try {
			return await callback()
		} catch (error) {
			errorMessage = formatError(error)
			return null
		} finally {
			operation = ''
		}
	}

	async function refreshDevice() {
		const result = await runOperation('device', checkDevice)

		if (result) {
			device = result
		}
	}

	async function readDeviceInfo() {
		const result = await runOperation('device-info', getDeviceInfo)

		if (result) {
			deviceInfo = result
			successMessage = 'Device information loaded.'
		}
	}

	function validateEnrollment() {
		const errors = {}
		const username = enrollmentForm.username.trim()
		const finger = enrollmentForm.finger.trim()
		const samples = Number(enrollmentForm.samples)
		const timeoutSecs = Number(enrollmentForm.timeoutSecs)

		if (!username) {
			errors.username = 'Username is required.'
		} else if (username.length > 128) {
			errors.username = 'Username cannot exceed 128 characters.'
		} else if (!/^[\p{L}\p{N}_-]+$/u.test(username)) {
			errors.username = 'Use only letters, numbers, hyphens, and underscores.'
		}

		if (!finger) {
			errors.finger = 'Select a finger.'
		}

		if (!Number.isInteger(samples) || samples < 1 || samples > 10) {
			errors.samples = 'Samples must be a whole number from 1 to 10.'
		}

		if (!Number.isInteger(timeoutSecs) || timeoutSecs < 1 || timeoutSecs > 300) {
			errors.timeoutSecs = 'Timeout must be a whole number from 1 to 300 seconds.'
		}

		enrollmentErrors = errors
		return Object.keys(errors).length === 0
	}

	async function enrollFingerprint() {
		if (!validateEnrollment()) {
			return
		}

		enrollment = null
		verification = null
		identification = null

		const result = await runOperation('enroll', () =>
			enroll({
				username: enrollmentForm.username.trim(),
				finger: enrollmentForm.finger,
				samples: Number(enrollmentForm.samples),
				timeoutSecs: Number(enrollmentForm.timeoutSecs),
				includeImages: true,
				saveLocally: enrollmentForm.saveLocally,
			}),
		)

		if (result) {
			enrollment = result
			verifyForm.username = result.username
			verifyForm.source = enrollmentForm.saveLocally ? 'local' : 'template'
			successMessage = `Enrollment completed with ${result.minutiaeCount} minutiae.`
		}
	}

	function validateVerification() {
		const errors = {}
		const threshold = Number(verifyForm.threshold)
		const timeoutSecs = Number(verifyForm.timeoutSecs)

		if (verifyForm.source === 'local' && !verifyForm.username.trim()) {
			errors.username = 'Username is required for a local verification.'
		}

		if (verifyForm.source === 'template' && !enrollment?.template) {
			errors.source = 'Enroll a fingerprint before using the in-memory template.'
		}

		if (!Number.isInteger(threshold) || threshold < 1 || threshold > 100) {
			errors.threshold = 'Sensitivity threshold must be from 1 to 100.'
		}

		if (!Number.isInteger(timeoutSecs) || timeoutSecs < 1 || timeoutSecs > 300) {
			errors.timeoutSecs = 'Timeout must be a whole number from 1 to 300 seconds.'
		}

		verifyErrors = errors
		return Object.keys(errors).length === 0
	}

	async function verifyFingerprint() {
		if (!validateVerification()) {
			return
		}

		verification = null
		const options = {
			threshold: Number(verifyForm.threshold),
			timeoutSecs: Number(verifyForm.timeoutSecs),
		}

		if (verifyForm.source === 'template') {
			options.template = enrollment.template
		} else {
			options.username = verifyForm.username.trim()
		}

		const result = await runOperation('verify', () => verify(options))

		if (result) {
			verification = result
			successMessage = result.matched
				? `Fingerprint matched with a score of ${result.score}.`
				: `Fingerprint did not match. Score: ${result.score}.`
		}
	}

	async function identifyFingerprint() {
		identification = null
		const result = await runOperation('identify', () =>
			identify({
				threshold: Number(verifyForm.threshold),
				timeoutSecs: Number(verifyForm.timeoutSecs),
			}),
		)

		if (result) {
			identification = result
			successMessage = result.matched
				? `Identified ${result.username} with a score of ${result.score}.`
				: `No match found in ${result.galleryCount} local templates.`
		}
	}

	async function captureDiagnostic() {
		captureResult = null
		const result = await runOperation('capture', () =>
			capture({
				timeoutSecs: Number(enrollmentForm.timeoutSecs),
			}),
		)

		if (result) {
			captureResult = result
			successMessage = `Diagnostic image saved with ${result.minutiaeCount} minutiae.`
		}
	}
</script>

<svelte:head>
	<title>Fingerprint Lab</title>
</svelte:head>

<header class="topbar">
	<div class="brand">
		<div class="brand-mark" aria-hidden="true">FP</div>
		<div>
			<strong>Fingerprint Lab</strong>
			<span>DigitalPersona U.are.U 4500</span>
		</div>
	</div>

	<div class:online={device?.detected} class="device-status">
		<span class="status-dot"></span>
		{#if operation === 'device'}
			Checking reader
		{:else if device?.detected}
			Reader connected
		{:else}
			Reader unavailable
		{/if}
	</div>
</header>

<main class="page-shell">
	<section class="hero">
		<div>
			<p class="eyebrow">Tauri plugin example</p>
			<h1>Enroll and verify fingerprints with confidence.</h1>
			<p class="hero-copy">
				A complete test surface for device health, enrollment quality,
				fingerprint previews, matching sensitivity, and local identification.
			</p>
		</div>

		<div class="hero-actions">
			<button class="button secondary" onclick={refreshDevice} disabled={operation !== ''}>
				Refresh reader
			</button>
			<button
				class="button ghost"
				onclick={readDeviceInfo}
				disabled={operation !== '' || !device?.detected}
			>
				Device details
			</button>
		</div>
	</section>

	{#if errorMessage}
		<div class="notice error" role="alert">
			<strong>Operation failed</strong>
			<span>{errorMessage}</span>
		</div>
	{/if}

	{#if successMessage}
		<div class="notice success" role="status">
			<strong>Success</strong>
			<span>{successMessage}</span>
		</div>
	{/if}

	{#if deviceInfo}
		<section class="device-grid" aria-label="Device information">
			<div>
				<span>Model</span>
				<strong>{deviceInfo.model}</strong>
			</div>
			<div>
				<span>Firmware</span>
				<strong>{deviceInfo.hardwareFirmwareVersion}</strong>
			</div>
			<div>
				<span>Image</span>
				<strong>{deviceInfo.imageWidth} × {deviceInfo.imageHeight}</strong>
			</div>
			<div>
				<span>Resolution</span>
				<strong>{deviceInfo.imagePpi} PPI</strong>
			</div>
		</section>
	{/if}

	<div class="workspace-grid">
		<section class="panel">
			<div class="panel-heading">
				<div>
					<p class="step">Step 1</p>
					<h2>Enroll fingerprint</h2>
					<p>Capture multiple samples and review every selected scan.</p>
				</div>
				<span class="panel-number">01</span>
			</div>

			<form onsubmit={(event) => { event.preventDefault(); void enrollFingerprint() }}>
				<div class="field-grid">
					<label class="field">
						<span>Username</span>
						<input
							class:invalid={enrollmentErrors.username}
							bind:value={enrollmentForm.username}
							placeholder="e.g. alfian_01"
							autocomplete="off"
						/>
						{#if enrollmentErrors.username}<small class="field-error">{enrollmentErrors.username}</small>{/if}
					</label>

					<label class="field">
						<span>Finger</span>
						<select class:invalid={enrollmentErrors.finger} bind:value={enrollmentForm.finger}>
							{#each fingers as finger}
								<option value={finger.value}>{finger.label}</option>
							{/each}
						</select>
						{#if enrollmentErrors.finger}<small class="field-error">{enrollmentErrors.finger}</small>{/if}
					</label>

					<label class="field">
						<span>Enrollment samples</span>
						<input
							class:invalid={enrollmentErrors.samples}
							type="number"
							min="1"
							max="10"
							bind:value={enrollmentForm.samples}
						/>
						<small>Four samples provide a good default balance.</small>
						{#if enrollmentErrors.samples}<small class="field-error">{enrollmentErrors.samples}</small>{/if}
					</label>

					<label class="field">
						<span>Scan timeout</span>
						<div class="input-suffix">
							<input
								class:invalid={enrollmentErrors.timeoutSecs}
								type="number"
								min="1"
								max="300"
								bind:value={enrollmentForm.timeoutSecs}
							/>
							<span>seconds</span>
						</div>
						{#if enrollmentErrors.timeoutSecs}<small class="field-error">{enrollmentErrors.timeoutSecs}</small>{/if}
					</label>
				</div>

				<label class="check-row">
					<input type="checkbox" bind:checked={enrollmentForm.saveLocally} />
					<span>
						<strong>Save template locally</strong>
						<small>Allows verification by username and local identification.</small>
					</span>
				</label>

				<button
					class="button primary full"
					type="submit"
					disabled={operation !== '' || !device?.detected}
				>
					{operation === 'enroll' ? 'Scanning fingerprint…' : 'Start enrollment'}
				</button>
			</form>

			{#if !device?.detected && operation !== 'device'}
				<p class="inline-hint">Connect the fingerprint reader before starting enrollment.</p>
			{/if}
		</section>

		<section class="panel preview-panel">
			<div class="panel-heading">
				<div>
					<p class="step">Live result</p>
					<h2>Scan preview</h2>
					<p>The plugin returns one PNG data URL for every completed scan.</p>
				</div>
				<span class="panel-number">02</span>
			</div>

			{#if operation === 'enroll'}
				<div class="scanner-state">
					<div class="fingerprint-glyph scanning" aria-hidden="true">◎</div>
					<strong>Follow the reader prompts</strong>
					<p>Place and remove the selected finger until every sample is captured.</p>
				</div>
			{:else if enrollment?.images.length}
				<div class="preview-grid">
					{#each enrollment.images as image, index}
						<figure>
							<img src={image} alt={`Fingerprint scan ${index + 1}`} />
							<figcaption>Sample {index + 1}</figcaption>
						</figure>
					{/each}
				</div>

				<div class="metrics-grid">
					<div><strong>{enrollment.scanCount}</strong><span>Scans</span></div>
					<div><strong>{enrollment.templateSampleCount}</strong><span>Template samples</span></div>
					<div><strong>{enrollment.minutiaeCount}</strong><span>Minutiae</span></div>
				</div>
			{:else}
				<div class="empty-state">
					<div class="fingerprint-glyph" aria-hidden="true">◎</div>
					<strong>No fingerprint captured yet</strong>
					<p>Complete an enrollment to see the selected fingerprint images here.</p>
				</div>
			{/if}
		</section>
	</div>

	<section class="panel verification-panel">
		<div class="panel-heading compact">
			<div>
				<p class="step">Step 2</p>
				<h2>Verify fingerprint</h2>
				<p>Choose the template source and tune the matching sensitivity.</p>
			</div>
			<span class="panel-number">03</span>
		</div>

		<div class="verify-layout">
			<form onsubmit={(event) => { event.preventDefault(); void verifyFingerprint() }}>
				<div class="source-switch" aria-label="Template source">
					<label class:active={verifyForm.source === 'local'}>
						<input type="radio" bind:group={verifyForm.source} value="local" />
						<span>Local template</span>
					</label>
					<label class:active={verifyForm.source === 'template'} class:disabled={!enrollment}>
						<input
							type="radio"
							bind:group={verifyForm.source}
							value="template"
							disabled={!enrollment}
						/>
						<span>Current enrollment</span>
					</label>
				</div>

				{#if verifyErrors.source}<small class="field-error block-error">{verifyErrors.source}</small>{/if}

				{#if verifyForm.source === 'local'}
					<label class="field">
						<span>Username</span>
						<input
							class:invalid={verifyErrors.username}
							bind:value={verifyForm.username}
							placeholder="Local template username"
						/>
						{#if verifyErrors.username}<small class="field-error">{verifyErrors.username}</small>{/if}
					</label>
				{/if}

				<label class="field sensitivity-field">
					<span>Matching sensitivity</span>
					<div class="range-heading">
						<strong>{sensitivityLabel(verifyForm.threshold)}</strong>
						<output>{verifyForm.threshold}</output>
					</div>
					<input
						type="range"
						min="1"
						max="100"
						step="1"
						bind:value={verifyForm.threshold}
					/>
					<div class="range-labels"><span>More permissive</span><span>More strict</span></div>
					<small>A higher threshold requires a stronger BOZORTH3 match.</small>
					{#if verifyErrors.threshold}<small class="field-error">{verifyErrors.threshold}</small>{/if}
				</label>

				<label class="field compact-field">
					<span>Scan timeout</span>
					<div class="input-suffix">
						<input
							class:invalid={verifyErrors.timeoutSecs}
							type="number"
							min="1"
							max="300"
							bind:value={verifyForm.timeoutSecs}
						/>
						<span>seconds</span>
					</div>
					{#if verifyErrors.timeoutSecs}<small class="field-error">{verifyErrors.timeoutSecs}</small>{/if}
				</label>

				<div class="button-row">
					<button class="button primary" type="submit" disabled={operation !== '' || !device?.detected}>
						{operation === 'verify' ? 'Verifying…' : 'Verify 1:1'}
					</button>
					<button
						class="button secondary"
						type="button"
						onclick={identifyFingerprint}
						disabled={operation !== '' || !device?.detected}
					>
						{operation === 'identify' ? 'Identifying…' : 'Identify 1:N'}
					</button>
				</div>
			</form>

			<div class="match-result" class:matched={verification?.matched || identification?.matched}>
				{#if operation === 'verify' || operation === 'identify'}
					<div class="result-icon waiting">◎</div>
					<strong>Waiting for fingerprint</strong>
					<p>Place your finger flat on the reader.</p>
				{:else if verification}
					<div class="result-icon">{verification.matched ? '✓' : '×'}</div>
					<strong>{verification.matched ? 'Fingerprint matched' : 'No match'}</strong>
					<p>{verification.username} · {verification.finger}</p>
					<div class="score-row">
						<span>Score <b>{verification.score}</b></span>
						<span>Threshold <b>{verification.threshold}</b></span>
					</div>
				{:else if identification}
					<div class="result-icon">{identification.matched ? '✓' : '×'}</div>
					<strong>{identification.matched ? 'Identity found' : 'No identity found'}</strong>
					<p>{identification.username || 'No matching local template'}</p>
					<div class="score-row">
						<span>Score <b>{identification.score ?? '—'}</b></span>
						<span>Gallery <b>{identification.galleryCount}</b></span>
					</div>
				{:else}
					<div class="result-icon idle">◎</div>
					<strong>Ready to verify</strong>
					<p>Your match result and score will appear here.</p>
				{/if}
			</div>
		</div>
	</section>

	<div class="bottom-grid">
		<section class="panel template-panel">
			<div class="panel-heading compact">
				<div>
					<p class="step">Server payload</p>
					<h2>Enrollment template</h2>
					<p>Store this JSON as sensitive biometric data, not the preview image.</p>
				</div>
			</div>

			{#if enrollment}
				<pre>{formatJson(enrollment.template)}</pre>
			{:else}
				<div class="small-empty">The JSON template will appear after enrollment.</div>
			{/if}
		</section>

		<section class="panel diagnostics-panel">
			<div class="panel-heading compact">
				<div>
					<p class="step">Diagnostics</p>
					<h2>Capture raw scan</h2>
					<p>Save a PGM image and inspect the detected minutiae count.</p>
				</div>
			</div>

			<button
				class="button secondary full"
				onclick={captureDiagnostic}
				disabled={operation !== '' || !device?.detected}
			>
				{operation === 'capture' ? 'Capturing…' : 'Capture diagnostic image'}
			</button>

			{#if captureResult}
				<dl class="diagnostic-result">
					<div><dt>Minutiae</dt><dd>{captureResult.minutiaeCount}</dd></div>
					<div><dt>Dimensions</dt><dd>{captureResult.imageWidth} × {captureResult.imageHeight}</dd></div>
					<div class="path-row"><dt>Saved path</dt><dd>{captureResult.path}</dd></div>
				</dl>
			{/if}
		</section>
	</div>
</main>

<footer>
	<span>tauri-plugin-fingerprint</span>
	<span>DigitalPersona U.are.U 4500 · Tauri 2</span>
</footer>
