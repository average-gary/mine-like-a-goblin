/**
 * Goblin Mining Co. - Hand-Cranked Bitcoin Mining Application
 */

// WASM module
let wasm = null;
let miner = null;
let api = null;

// State
let currentNetwork = 'mainnet';
let miningInterval = null;
let blockCheckInterval = null;
let isInitialized = false;
let miningStartTime = null;
let totalSwings = 0;
let sharesFound = 0;
let bestHash = null;
let bestLeadingZeros = 0;
let isAutoMining = false;
let networkStats = null;
let currentBlockHeight = null;

// DOM Elements
const elements = {
    // Network toggle
    mainnetBtn: document.getElementById('mainnet-btn'),
    testnetBtn: document.getElementById('testnet-btn'),
    networkDisplay: document.getElementById('network-display'),

    // Address input
    addressInput: document.getElementById('address-input'),
    addressFeedback: document.getElementById('address-feedback'),

    // Nonce input
    nonceInput: document.getElementById('nonce-input'),

    // Block info
    blockHeight: document.getElementById('block-height'),
    infoHeight: document.getElementById('info-height'),
    infoDifficulty: document.getElementById('info-difficulty'),
    infoPrevHash: document.getElementById('info-prev-hash'),

    // Prize
    prizeAmount: document.getElementById('prize-amount'),
    prizeSats: document.getElementById('prize-sats'),

    // Stats
    hashRate: document.getElementById('hash-rate'),
    totalHashes: document.getElementById('total-hashes'),
    sharesFound: document.getElementById('shares-found'),
    currentNonce: document.getElementById('current-nonce'),
    elapsedTime: document.getElementById('elapsed-time'),
    bestHash: document.getElementById('best-hash'),

    // Controls
    startBtn: document.getElementById('start-btn'),
    stopBtn: document.getElementById('stop-btn'),
    autoControls: document.getElementById('auto-controls'),
    autoMineToggle: document.getElementById('auto-mine-toggle'),

    // Pickaxe
    pickaxeBtn: document.getElementById('pickaxe-btn'),
    miningArea: document.getElementById('mining-area'),
    rockFace: document.getElementById('rock-face'),
    sparks: document.getElementById('sparks'),

    // Modal
    winnerModal: document.getElementById('winner-modal'),
    winnerHash: document.getElementById('winner-hash'),
    winnerNonce: document.getElementById('winner-nonce'),
    submitBlockBtn: document.getElementById('submit-block-btn'),
    closeModalBtn: document.getElementById('close-modal-btn'),

    // Toast
    shareToast: document.getElementById('share-toast'),
    shareZeros: document.getElementById('share-zeros'),

    // Ticket/Claim
    lotteryTicket: document.getElementById('lottery-ticket'),
    hashDisplay: document.getElementById('hash-display'),

    // Network stats
    networkHashrate: document.getElementById('network-hashrate'),
    networkDifficulty: document.getElementById('network-difficulty'),
    goblinShare: document.getElementById('goblin-share'),
    poolsList: document.getElementById('pools-list'),
};

/**
 * Initialize the application
 */
async function init() {
    log('Initializing Goblin Mining Co...');

    try {
        // Load WASM module
        await loadWasm();

        // Set up event listeners
        setupEventListeners();

        // Set default network
        setNetwork('mainnet');

        // Fetch network stats from mempool.space
        fetchNetworkStats();

        // Refresh network stats every 60 seconds
        setInterval(fetchNetworkStats, 60000);

        log('Initialization complete!', 'success');
        log('Enter your Bitcoin address and nonce to begin swinging!');

    } catch (error) {
        log(`Initialization failed: ${error.message}`, 'error');
        console.error(error);
    }
}

/**
 * Fetch network stats from mempool.space
 */
async function fetchNetworkStats() {
    try {
        // Fetch hashrate data
        const hashrateResponse = await fetch('https://mempool.space/api/v1/mining/hashrate/1w');
        const hashrateData = await hashrateResponse.json();

        // Fetch pools data
        const poolsResponse = await fetch('https://mempool.space/api/v1/mining/pools/1w');
        const poolsData = await poolsResponse.json();

        networkStats = {
            hashrate: hashrateData.currentHashrate,
            difficulty: hashrateData.currentDifficulty,
            pools: poolsData.pools.slice(0, 3)
        };

        updateNetworkStatsDisplay();

    } catch (error) {
        console.warn('Failed to fetch network stats:', error);
        elements.networkHashrate.textContent = 'Unavailable';
        elements.networkDifficulty.textContent = 'Unavailable';
    }
}

/**
 * Update network stats display
 */
function updateNetworkStatsDisplay() {
    if (!networkStats) return;

    // Format hashrate (EH/s)
    const hashrateEH = networkStats.hashrate / 1e18;
    elements.networkHashrate.textContent = `${hashrateEH.toFixed(2)} EH/s`;

    // Format difficulty (T)
    const difficultyT = networkStats.difficulty / 1e12;
    elements.networkDifficulty.textContent = `${difficultyT.toFixed(2)} T`;

    // Calculate goblin's share (humorous, human-readable)
    const goblinHashrate = isAutoMining ? 1000000 : (totalSwings > 0 ? 1 : 0); // ~1 MH/s when auto-mining
    if (goblinHashrate > 0) {
        const ratio = networkStats.hashrate / goblinHashrate;
        elements.goblinShare.textContent = `1 in ${formatLargeNumber(ratio)}`;
    } else {
        elements.goblinShare.textContent = '0';
    }

    // Update pools list
    if (networkStats.pools && networkStats.pools.length > 0) {
        elements.poolsList.innerHTML = networkStats.pools.map((pool, index) => `
            <div class="pool-item">
                <span>
                    <span class="pool-name">${pool.name}</span>
                    <span class="pool-rank">#${index + 1}</span>
                </span>
                <span class="pool-blocks">${pool.blockCount} blocks</span>
            </div>
        `).join('');
    }
}

/**
 * Load the WASM module
 */
async function loadWasm() {
    try {
        // Load WASM from pkg directory (copied into web/ for deployment)
        wasm = await import('./pkg/miner_wasm.js');
        await wasm.default();

        log(`WASM loaded (v${wasm.version()})`);
        isInitialized = true;
    } catch (error) {
        // For development without WASM, use mock
        log('WASM not available, using mock mode', 'error');
        console.warn('WASM load failed:', error);
        useMockMode();
    }
}

/**
 * Mock mode for development without WASM
 */
function useMockMode() {
    wasm = {
        version: () => '0.1.0-mock',
        Miner: class MockMiner {
            constructor(address, network) {
                this.network = network;
                this.address = address;
                this.stats = { total_hashes: 0, shares_found: 0 };
                this._nonce = 0;
            }
            static validate_address(address, network) {
                if (address.startsWith('bc1') && network === 'mainnet') return true;
                if (address.startsWith('tb1') && network === 'testnet4') return true;
                if (address.startsWith('1') && network === 'mainnet') return true;
                throw new Error('Invalid address');
            }
            build_template() {
                return {
                    height: 875000,
                    prev_hash: '0000000000000000000000000000000000000000000000000000000000000000',
                    difficulty: 90000000000000,
                    difficulty_display: '90.00T',
                    reward: 312500000,
                    reward_btc: 3.125,
                };
            }
            mine_single_nonce(nonce) {
                this._nonce = nonce;
                this.stats.total_hashes++;
                // Simulate hash result - very small chance of share
                const shareFound = Math.random() < 0.004; // ~1 in 256 for share
                if (shareFound) this.stats.shares_found++;

                // Generate mock hash
                const prefix = shareFound ? '00' : Math.random().toString(16).slice(2, 4);
                const hash = prefix + Math.random().toString(16).slice(2).padEnd(62, '0');

                return {
                    share_found: shareFound,
                    block_found: false,
                    hash: hash,
                    leading_zeros: shareFound ? 8 : 0,
                    nonce: nonce,
                };
            }
            mine_batch(size) {
                this.stats.total_hashes += size;
                const shareFound = Math.random() < 0.001;
                if (shareFound) this.stats.shares_found++;
                return {
                    share_found: shareFound,
                    block_found: false,
                    hash: shareFound ? '0000' + Math.random().toString(16).slice(2, 62) : null,
                    leading_zeros: shareFound ? 16 : 0,
                    hashes_computed: size,
                };
            }
            get_stats() {
                return {
                    total_hashes: this.stats.total_hashes,
                    hash_rate: this.stats.total_hashes / 10,
                    shares_found: this.stats.shares_found,
                    current_nonce: this._nonce,
                };
            }
            start_mining() {}
            stop_mining() {}
            reset() {
                this.stats = { total_hashes: 0, shares_found: 0 };
            }
        },
        BlockchainApi: class MockApi {
            constructor(network) {
                this.network = network;
            }
            async get_tip_hash() {
                return '0000000000000000000' + Math.random().toString(16).slice(2, 45);
            }
            async get_tip_height() {
                return currentNetwork === 'mainnet' ? 875000 : 50000;
            }
        }
    };
    isInitialized = true;
}

/**
 * Set up event listeners
 */
function setupEventListeners() {
    // Network toggle
    elements.mainnetBtn.addEventListener('click', () => setNetwork('mainnet'));
    elements.testnetBtn.addEventListener('click', () => setNetwork('testnet4'));

    // Address input
    elements.addressInput.addEventListener('input', debounce(validateAddress, 300));
    elements.addressInput.addEventListener('blur', validateAddress);

    // Nonce input - enable pickaxe when valid
    elements.nonceInput.addEventListener('input', validateNonceInput);

    // Pickaxe button - the main mining action!
    elements.pickaxeBtn.addEventListener('click', swingPickaxe);

    // Auto-mine toggle
    elements.autoMineToggle.addEventListener('change', toggleAutoMine);

    // Mining controls (for auto-mine)
    elements.startBtn.addEventListener('click', startAutoMining);
    elements.stopBtn.addEventListener('click', stopAutoMining);

    // Modal
    elements.closeModalBtn.addEventListener('click', closeWinnerModal);
    elements.submitBlockBtn.addEventListener('click', submitBlock);
}

/**
 * Validate nonce input and enable/disable pickaxe
 */
function validateNonceInput() {
    const nonceValue = elements.nonceInput.value.trim();
    const addressValid = elements.addressInput.classList.contains('valid');

    if (nonceValue !== '' && addressValid) {
        const nonce = parseInt(nonceValue, 10);
        if (!isNaN(nonce) && nonce >= 0 && nonce <= 4294967295) {
            elements.pickaxeBtn.disabled = false;
            return;
        }
    }

    elements.pickaxeBtn.disabled = true;
}

/**
 * Swing the pickaxe! (Manual mining)
 */
async function swingPickaxe() {
    if (!miner) {
        log('Please enter a valid address first', 'error');
        return;
    }

    const nonceValue = elements.nonceInput.value.trim();
    if (nonceValue === '') {
        log('Please enter a nonce value', 'error');
        return;
    }

    const nonce = parseInt(nonceValue, 10);
    if (isNaN(nonce) || nonce < 0 || nonce > 4294967295) {
        log('Invalid nonce - must be 0 to 4,294,967,295', 'error');
        return;
    }

    // Start timing if this is first swing
    if (!miningStartTime) {
        miningStartTime = Date.now();
    }

    // Animate pickaxe
    elements.pickaxeBtn.classList.add('swinging');
    setTimeout(() => elements.pickaxeBtn.classList.remove('swinging'), 300);

    // Create sparks
    createSparks();

    // Add crack effect to rock
    elements.rockFace.classList.add('cracked');
    setTimeout(() => elements.rockFace.classList.remove('cracked'), 500);

    try {
        // Mine with the specific nonce
        let result;
        if (miner.mine_single_nonce) {
            result = miner.mine_single_nonce(nonce);
        } else {
            // Fallback for WASM that doesn't have single nonce method
            result = miner.mine_batch(1);
            result.nonce = nonce;
        }

        totalSwings++;

        // Update display
        elements.currentNonce.textContent = formatNumber(nonce);
        elements.totalHashes.textContent = formatNumber(totalSwings);
        elements.hashDisplay.textContent = result.hash || 'No result';

        // Update elapsed time
        const elapsedMs = Date.now() - miningStartTime;
        elements.elapsedTime.textContent = formatTime(elapsedMs);

        // Check for share
        if (result.share_found) {
            sharesFound++;
            elements.sharesFound.textContent = sharesFound;
            showShareToast(result.leading_zeros);
            elements.lotteryTicket.classList.add('share-found');
            setTimeout(() => elements.lotteryTicket.classList.remove('share-found'), 300);
            log(`Share found with nonce ${nonce}! Leading zeros: ${result.leading_zeros}`, 'share');
        }

        // Update best hash
        const leadingZeros = countLeadingZeros(result.hash);
        if (leadingZeros > bestLeadingZeros) {
            bestLeadingZeros = leadingZeros;
            bestHash = result.hash;
            elements.bestHash.textContent = bestHash.slice(0, 20) + '...';
        }

        // Check for block (extremely unlikely!)
        if (result.block_found) {
            elements.lotteryTicket.classList.add('winner');
            elements.hashDisplay.classList.add('winner');
            elements.winnerHash.textContent = result.hash;
            elements.winnerNonce.textContent = nonce;
            showWinnerModal();
            log('BLOCK FOUND! THE GOBLINS REJOICE!', 'success');
            createConfetti();
        }

        // Update goblin share display
        updateNetworkStatsDisplay();

    } catch (error) {
        log(`Mining error: ${error.message}`, 'error');
    }
}

/**
 * Create spark effects
 */
function createSparks() {
    const sparkCount = 8;
    elements.sparks.innerHTML = '';

    for (let i = 0; i < sparkCount; i++) {
        const spark = document.createElement('div');
        spark.className = 'spark';

        // Random direction
        const angle = (Math.PI * 2 * i) / sparkCount + (Math.random() - 0.5) * 0.5;
        const distance = 30 + Math.random() * 40;
        const tx = Math.cos(angle) * distance;
        const ty = Math.sin(angle) * distance - 20; // Bias upward

        spark.style.setProperty('--tx', tx + 'px');
        spark.style.setProperty('--ty', ty + 'px');

        elements.sparks.appendChild(spark);
    }

    // Clean up sparks after animation
    setTimeout(() => {
        elements.sparks.innerHTML = '';
    }, 500);
}

/**
 * Count leading zero bits in a hex hash
 */
function countLeadingZeros(hash) {
    if (!hash) return 0;
    let zeros = 0;
    for (const char of hash) {
        const nibble = parseInt(char, 16);
        if (nibble === 0) {
            zeros += 4;
        } else {
            // Count leading zeros in nibble
            if (nibble < 8) zeros++;
            if (nibble < 4) zeros++;
            if (nibble < 2) zeros++;
            break;
        }
    }
    return zeros;
}

/**
 * Toggle auto-mine mode
 */
function toggleAutoMine() {
    isAutoMining = elements.autoMineToggle.checked;

    if (isAutoMining) {
        elements.autoControls.style.display = 'flex';
        elements.pickaxeBtn.disabled = true;
        elements.nonceInput.disabled = true;

        // Enable start button if address is valid
        if (elements.addressInput.classList.contains('valid')) {
            elements.startBtn.disabled = false;
        }
    } else {
        elements.autoControls.style.display = 'none';
        stopAutoMining();
        elements.nonceInput.disabled = false;
        validateNonceInput();
    }
}

/**
 * Start auto-mining
 */
function startAutoMining() {
    if (!miner) {
        log('Please enter a valid address first', 'error');
        return;
    }

    log('Starting auto-mining (betraying goblin principles)...');

    miner.start_mining();
    miningStartTime = Date.now();

    // Update UI
    elements.startBtn.disabled = true;
    elements.stopBtn.disabled = false;
    elements.addressInput.disabled = true;
    elements.lotteryTicket.classList.add('mining-active');

    // Start mining loop
    miningInterval = setInterval(autoMiningLoop, 100);

    log('Auto-mining started!', 'success');
}

/**
 * Stop auto-mining
 */
function stopAutoMining() {
    if (miningInterval) {
        clearInterval(miningInterval);
        miningInterval = null;
    }

    if (miner) {
        miner.stop_mining();
    }

    // Update UI
    elements.startBtn.disabled = false;
    elements.stopBtn.disabled = true;
    elements.addressInput.disabled = false;
    elements.lotteryTicket.classList.remove('mining-active');

    log('Auto-mining stopped');
}

/**
 * Auto-mining loop
 */
function autoMiningLoop() {
    if (!miner) return;

    try {
        // Mine a batch of nonces
        const result = miner.mine_batch(100000);

        // Update stats
        const stats = miner.get_stats();
        totalSwings = stats.total_hashes || 0;

        elements.hashRate.textContent = formatHashRate(stats.hash_rate || 0);
        elements.totalHashes.textContent = formatNumber(totalSwings);
        elements.sharesFound.textContent = stats.shares_found || 0;
        elements.currentNonce.textContent = formatNumber(stats.current_nonce || 0);

        const elapsedMs = Date.now() - miningStartTime;
        elements.elapsedTime.textContent = formatTime(elapsedMs);

        // Update hash display
        if (stats.best_hash) {
            elements.hashDisplay.textContent = stats.best_hash;
            elements.bestHash.textContent = stats.best_hash.slice(0, 20) + '...';
        }

        // Handle share found
        if (result.share_found && !result.block_found) {
            showShareToast(result.leading_zeros);
            elements.lotteryTicket.classList.add('share-found');
            setTimeout(() => {
                elements.lotteryTicket.classList.remove('share-found');
            }, 300);
            log(`Share found! Leading zeros: ${result.leading_zeros}`, 'share');
        }

        // Handle block found
        if (result.block_found) {
            stopAutoMining();
            elements.lotteryTicket.classList.add('winner');
            elements.hashDisplay.classList.add('winner');
            elements.hashDisplay.textContent = result.hash;
            elements.winnerHash.textContent = result.hash;
            elements.winnerNonce.textContent = result.nonce;
            showWinnerModal();
            log('BLOCK FOUND! THE GOBLINS REJOICE!', 'success');
            createConfetti();
        }

        // Update network stats display
        updateNetworkStatsDisplay();

    } catch (error) {
        log(`Mining error: ${error.message}`, 'error');
        stopAutoMining();
    }
}

/**
 * Set the current network
 */
function setNetwork(network) {
    currentNetwork = network;

    // Update UI
    elements.mainnetBtn.classList.toggle('active', network === 'mainnet');
    elements.testnetBtn.classList.toggle('active', network === 'testnet4');
    elements.networkDisplay.textContent = network.toUpperCase();
    elements.networkDisplay.classList.toggle('testnet', network === 'testnet4');

    // Update placeholder
    if (network === 'mainnet') {
        elements.addressInput.placeholder = 'bc1q... or 1... or 3...';
    } else {
        elements.addressInput.placeholder = 'tb1q... or m... or 2...';
    }

    // Reset miner if address is set
    if (elements.addressInput.value) {
        validateAddress();
    }

    // Update prize display
    updatePrizeDisplay();

    log(`Switched to ${network}`);
}

/**
 * Update prize display based on network and height
 */
function updatePrizeDisplay(height = null) {
    // Calculate subsidy based on height
    const halvings = height ? Math.floor(height / 210000) : 4;
    const subsidySats = Math.floor(5000000000 / Math.pow(2, halvings));
    const subsidyBtc = subsidySats / 100000000;

    elements.prizeAmount.textContent = `${subsidyBtc.toFixed(3)} BTC`;
    elements.prizeSats.textContent = `(${subsidySats.toLocaleString()} sats)`;
}

/**
 * Validate the address input
 */
function validateAddress() {
    const address = elements.addressInput.value.trim();

    if (!address) {
        elements.addressInput.classList.remove('valid', 'invalid');
        elements.addressFeedback.textContent = '';
        elements.addressFeedback.classList.remove('valid', 'invalid');
        elements.pickaxeBtn.disabled = true;
        elements.startBtn.disabled = true;
        return;
    }

    try {
        wasm.Miner.validate_address(address, currentNetwork);

        elements.addressInput.classList.add('valid');
        elements.addressInput.classList.remove('invalid');
        elements.addressFeedback.textContent = 'Valid address';
        elements.addressFeedback.classList.add('valid');
        elements.addressFeedback.classList.remove('invalid');

        // Create miner instance
        createMiner(address);

        // Enable appropriate controls
        if (isAutoMining) {
            elements.startBtn.disabled = false;
        } else {
            validateNonceInput();
        }

    } catch (error) {
        elements.addressInput.classList.add('invalid');
        elements.addressInput.classList.remove('valid');
        elements.addressFeedback.textContent = error.message || 'Invalid address';
        elements.addressFeedback.classList.add('invalid');
        elements.addressFeedback.classList.remove('valid');
        elements.pickaxeBtn.disabled = true;
        elements.startBtn.disabled = true;
    }
}

/**
 * Create a new miner instance
 */
async function createMiner(address) {
    try {
        miner = new wasm.Miner(address, currentNetwork);
        api = new wasm.BlockchainApi(currentNetwork);

        log(`Miner created for ${address.slice(0, 10)}...`);

        // Reset stats
        totalSwings = 0;
        sharesFound = 0;
        bestHash = null;
        bestLeadingZeros = 0;
        miningStartTime = null;

        // Fetch block template data
        await fetchBlockTemplate();

    } catch (error) {
        log(`Failed to create miner: ${error.message}`, 'error');
    }
}

/**
 * Fetch block template data from API
 */
async function fetchBlockTemplate() {
    try {
        log('Fetching block template...');

        const tipHash = await api.get_tip_hash();
        const tipHeight = await api.get_tip_height();

        // For bits, we'd normally get this from the API
        // Using a reasonable default for now
        const bits = currentNetwork === 'mainnet' ? 0x17034219 : 0x1d00ffff;
        const timestamp = Math.floor(Date.now() / 1000);

        const templateInfo = miner.build_template(tipHash, tipHeight, bits, timestamp);

        // Update UI
        elements.blockHeight.textContent = templateInfo.height;
        elements.infoHeight.textContent = templateInfo.height;
        elements.infoDifficulty.textContent = templateInfo.difficulty_display;
        elements.infoPrevHash.textContent = tipHash.slice(0, 16) + '...';

        // Store current height for block watching
        currentBlockHeight = tipHeight;

        updatePrizeDisplay(templateInfo.height);

        log(`Template ready for block ${templateInfo.height}`);

        // Start watching for new blocks if not already watching
        startBlockWatcher();

    } catch (error) {
        log(`Failed to fetch template: ${error.message}`, 'error');
    }
}

/**
 * Start watching for new blocks (every 30 seconds)
 */
function startBlockWatcher() {
    // Clear any existing watcher
    if (blockCheckInterval) {
        clearInterval(blockCheckInterval);
    }

    // Check for new blocks every 30 seconds
    blockCheckInterval = setInterval(checkForNewBlock, 30000);
    log('Block watcher started (checking every 30s)');
}

/**
 * Check if a new block has been mined
 */
async function checkForNewBlock() {
    if (!api || !miner) return;

    try {
        const tipHeight = await api.get_tip_height();

        if (tipHeight > currentBlockHeight) {
            log(`New block detected! Height: ${tipHeight}`, 'success');

            // Show notification to user
            showNewBlockNotification(tipHeight);

            // Refresh the block template
            await fetchBlockTemplate();

            // Reset mining stats for the new block
            if (!isAutoMining) {
                totalSwings = 0;
                sharesFound = 0;
                bestHash = null;
                bestLeadingZeros = 0;
                elements.totalHashes.textContent = '0';
                elements.sharesFound.textContent = '0';
                elements.bestHash.textContent = '-';
                elements.hashDisplay.textContent = 'New block! Enter a nonce and swing!';
            }
        }
    } catch (error) {
        console.warn('Failed to check for new block:', error);
    }
}

/**
 * Show notification when a new block is found
 */
function showNewBlockNotification(height) {
    // Reuse the share toast for new block notification
    elements.shareZeros.textContent = `Block ${height} mined!`;
    elements.shareToast.classList.add('active');
    elements.shareToast.style.background = '#cd7f32'; // Bronze color for block notification

    setTimeout(() => {
        elements.shareToast.classList.remove('active');
        elements.shareToast.style.background = ''; // Reset to default
    }, 3000);
}

/**
 * Show share found toast
 */
function showShareToast(leadingZeros) {
    elements.shareZeros.textContent = leadingZeros;
    elements.shareToast.classList.add('active');

    setTimeout(() => {
        elements.shareToast.classList.remove('active');
    }, 2000);
}

/**
 * Show winner modal
 */
function showWinnerModal() {
    elements.winnerModal.classList.add('active');
}

/**
 * Close winner modal
 */
function closeWinnerModal() {
    elements.winnerModal.classList.remove('active');
}

/**
 * Submit the found block
 */
async function submitBlock() {
    if (!miner) return;

    try {
        const blockHex = miner.get_block_hex();
        if (!blockHex) {
            log('No valid block to submit', 'error');
            return;
        }

        log('Submitting block...');
        const result = await api.submit_tx(blockHex);
        log(`Block submitted: ${result}`, 'success');

    } catch (error) {
        log(`Block submission failed: ${error.message}`, 'error');
    }

    closeWinnerModal();
}

/**
 * Create confetti effect for winners (steampunk style)
 */
function createConfetti() {
    const types = ['gear', 'spark', 'copper'];

    for (let i = 0; i < 50; i++) {
        const confetti = document.createElement('div');
        confetti.className = 'confetti ' + types[Math.floor(Math.random() * types.length)];
        confetti.style.left = Math.random() * 100 + '%';
        confetti.style.animationDelay = Math.random() * 2 + 's';
        document.body.appendChild(confetti);

        setTimeout(() => confetti.remove(), 5000);
    }
}

/**
 * Log a message to the console
 */
function log(message, type = 'info') {
    console.log(`[${type}] ${message}`);
}

/**
 * Format hash rate for display
 */
function formatHashRate(rate) {
    if (rate >= 1e9) return (rate / 1e9).toFixed(2) + ' GH/s';
    if (rate >= 1e6) return (rate / 1e6).toFixed(2) + ' MH/s';
    if (rate >= 1e3) return (rate / 1e3).toFixed(2) + ' KH/s';
    return rate.toFixed(0) + ' H/s';
}

/**
 * Format large numbers with names (trillion, quadrillion, etc.)
 */
function formatLargeNumber(num) {
    const names = [
        { value: 1e24, name: 'septillion' },
        { value: 1e21, name: 'sextillion' },
        { value: 1e18, name: 'quintillion' },
        { value: 1e15, name: 'quadrillion' },
        { value: 1e12, name: 'trillion' },
        { value: 1e9, name: 'billion' },
        { value: 1e6, name: 'million' },
        { value: 1e3, name: 'thousand' },
    ];

    for (const { value, name } of names) {
        if (num >= value) {
            const formatted = (num / value).toFixed(1);
            return `${formatted} ${name}`;
        }
    }

    return num.toFixed(0);
}

/**
 * Format number with commas
 */
function formatNumber(num) {
    return num.toLocaleString();
}

/**
 * Format elapsed time
 */
function formatTime(ms) {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
        return `${hours}:${(minutes % 60).toString().padStart(2, '0')}:${(seconds % 60).toString().padStart(2, '0')}`;
    }
    return `${minutes}:${(seconds % 60).toString().padStart(2, '0')}`;
}

/**
 * Debounce utility
 */
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', init);
