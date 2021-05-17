const videoBitsPerSecond = 5000000;
let recordedBlobs = [];
let supportedType = null;
let mediaRecorder = null;

const TYPES = {
	'video/mpeg': 'mp4',
	'video/webm': 'webm'
};

export function startRecording(canvas) {
	const stream = canvas.captureStream();
	if (!stream) {
		return;
	}

	for (let t of Object.keys(TYPES)) {
		if (MediaRecorder.isTypeSupported(t)) {
			supportedType = t;
			break;
		}
	}
	if (supportedType == null) {
		return;
	}
	let options = {
		mimeType: supportedType,
		videoBitsPerSecond
	};

	recordedBlobs = [];
	try {
		mediaRecorder = new MediaRecorder(stream, options);
	} catch (e) {
		console.error('Error creating MediaRecorder:', e);
		return;
	}

	mediaRecorder.onstop = event => {
		console.log('Recorder stopped: ', event);
	};
	mediaRecorder.ondataavailable = event => {
		if (event.data && event.data.size > 0) {
			recordedBlobs.push(event.data);
		}
	}
	mediaRecorder.start(1000); // ms per blob

	console.log('recording - started');
}

export function stopRecording() {
	if (!mediaRecorder) {
		return;
	}
	mediaRecorder.stop();
	console.log('Recorded Blobs: ', recordedBlobs);

	download(`recording-${Date.now()}`);

	supportedType = null;

	console.log('recording - stopped');
}

function download(filename) {
	const name = `${filename}.${TYPES[supportedType]}`;
	const blob = new Blob(recordedBlobs, {type: supportedType});
	const url = window.URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.style.display = 'none';
	a.href = url;
	a.download = name;
	document.body.appendChild(a);
	a.click();
	setTimeout(() => {
		document.body.removeChild(a);
		window.URL.revokeObjectURL(url);
	}, 100);
}
