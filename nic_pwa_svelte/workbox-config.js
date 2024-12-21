module.exports = {
	globDirectory: 'public/',
	globPatterns: [
		'**/*.{png,css,html}'
	],
	swDest: 'public/sw.js',
	ignoreURLParametersMatching: [
		/^utm_/,
		/^fbclid$/
	]
};