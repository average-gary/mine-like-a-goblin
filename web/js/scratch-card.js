/**
 * Scratch Card Component
 * Creates an interactive scratch-off effect using HTML5 Canvas
 */

export class ScratchCard {
    constructor(canvasId, options = {}) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d');

        // Options with defaults
        this.options = {
            scratchColor: '#c0c0c0',
            brushSize: 30,
            revealThreshold: 0.4, // Auto-reveal when 40% scratched
            onReveal: () => {},
            onScratchProgress: () => {},
            ...options
        };

        // State
        this.isScratching = false;
        this.scratchPercentage = 0;
        this.revealed = false;
        this.resultText = '';
        this.isWinner = false;

        // Bind methods
        this.handleStart = this.handleStart.bind(this);
        this.handleMove = this.handleMove.bind(this);
        this.handleEnd = this.handleEnd.bind(this);

        this.init();
    }

    init() {
        // Set canvas size to match display size
        const rect = this.canvas.getBoundingClientRect();
        this.canvas.width = rect.width;
        this.canvas.height = rect.height;

        // Draw initial scratch layer
        this.drawScratchLayer();

        // Add event listeners
        this.addEventListeners();
    }

    drawScratchLayer() {
        const { width, height } = this.canvas;

        // Fill with scratch color
        this.ctx.fillStyle = this.options.scratchColor;
        this.ctx.fillRect(0, 0, width, height);

        // Add lottery-style pattern
        this.ctx.fillStyle = '#a0a0a0';
        for (let i = 0; i < 50; i++) {
            const x = Math.random() * width;
            const y = Math.random() * height;
            const size = Math.random() * 3 + 1;
            this.ctx.beginPath();
            this.ctx.arc(x, y, size, 0, Math.PI * 2);
            this.ctx.fill();
        }

        // Add "SCRATCH HERE" text
        this.ctx.fillStyle = '#808080';
        this.ctx.font = 'bold 24px Arial';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText('SCRATCH HERE', width / 2, height / 2);

        // Add decorative border lines
        this.ctx.strokeStyle = '#909090';
        this.ctx.lineWidth = 2;
        this.ctx.setLineDash([5, 5]);
        this.ctx.strokeRect(10, 10, width - 20, height - 20);
        this.ctx.setLineDash([]);
    }

    addEventListeners() {
        // Mouse events
        this.canvas.addEventListener('mousedown', this.handleStart);
        this.canvas.addEventListener('mousemove', this.handleMove);
        this.canvas.addEventListener('mouseup', this.handleEnd);
        this.canvas.addEventListener('mouseleave', this.handleEnd);

        // Touch events
        this.canvas.addEventListener('touchstart', this.handleStart, { passive: false });
        this.canvas.addEventListener('touchmove', this.handleMove, { passive: false });
        this.canvas.addEventListener('touchend', this.handleEnd);
    }

    removeEventListeners() {
        this.canvas.removeEventListener('mousedown', this.handleStart);
        this.canvas.removeEventListener('mousemove', this.handleMove);
        this.canvas.removeEventListener('mouseup', this.handleEnd);
        this.canvas.removeEventListener('mouseleave', this.handleEnd);
        this.canvas.removeEventListener('touchstart', this.handleStart);
        this.canvas.removeEventListener('touchmove', this.handleMove);
        this.canvas.removeEventListener('touchend', this.handleEnd);
    }

    handleStart(e) {
        if (this.revealed) return;

        e.preventDefault();
        this.isScratching = true;
        this.canvas.parentElement.classList.add('scratching');
        this.scratch(e);
    }

    handleMove(e) {
        if (!this.isScratching || this.revealed) return;

        e.preventDefault();
        this.scratch(e);
    }

    handleEnd(e) {
        this.isScratching = false;
        this.canvas.parentElement.classList.remove('scratching');
    }

    scratch(e) {
        const pos = this.getPosition(e);

        // Use destination-out to "erase" the scratch layer
        this.ctx.globalCompositeOperation = 'destination-out';

        // Draw a circle at the scratch position
        this.ctx.beginPath();
        this.ctx.arc(pos.x, pos.y, this.options.brushSize, 0, Math.PI * 2);
        this.ctx.fill();

        // Reset composite operation
        this.ctx.globalCompositeOperation = 'source-over';

        // Update scratch percentage
        this.updateScratchPercentage();
    }

    getPosition(e) {
        const rect = this.canvas.getBoundingClientRect();

        if (e.touches) {
            return {
                x: e.touches[0].clientX - rect.left,
                y: e.touches[0].clientY - rect.top
            };
        }

        return {
            x: e.clientX - rect.left,
            y: e.clientY - rect.top
        };
    }

    updateScratchPercentage() {
        const imageData = this.ctx.getImageData(
            0, 0,
            this.canvas.width,
            this.canvas.height
        );

        let transparentPixels = 0;
        const totalPixels = imageData.data.length / 4;

        // Count transparent pixels (alpha channel is every 4th value)
        for (let i = 3; i < imageData.data.length; i += 4) {
            if (imageData.data[i] === 0) {
                transparentPixels++;
            }
        }

        this.scratchPercentage = transparentPixels / totalPixels;
        this.options.onScratchProgress(this.scratchPercentage);

        // Auto-reveal if threshold reached
        if (this.scratchPercentage >= this.options.revealThreshold && !this.revealed) {
            this.revealAll();
        }
    }

    revealAll() {
        if (this.revealed) return;

        this.revealed = true;
        this.canvas.parentElement.classList.add('revealed');

        // Clear the canvas completely
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

        // Remove event listeners
        this.removeEventListeners();

        // Trigger callback
        this.options.onReveal(this.isWinner);
    }

    setResult(hashHex, isWinner = false) {
        this.resultText = hashHex;
        this.isWinner = isWinner;

        // Update the result layer
        const resultLayer = document.getElementById('result-layer');
        const hashDisplay = document.getElementById('hash-display');

        if (hashDisplay) {
            if (isWinner) {
                hashDisplay.innerHTML = `
                    <div style="color: #ffd700; font-size: 1.5rem; margin-bottom: 10px;">
                        WINNER!
                    </div>
                    <div>${hashHex}</div>
                `;
                hashDisplay.classList.add('winner');
            } else {
                hashDisplay.textContent = hashHex || 'Mining in progress...';
                hashDisplay.classList.remove('winner');
            }
        }
    }

    reset() {
        this.revealed = false;
        this.scratchPercentage = 0;
        this.resultText = '';
        this.isWinner = false;

        this.canvas.parentElement.classList.remove('revealed', 'scratching');

        // Reinitialize
        this.drawScratchLayer();
        this.addEventListeners();

        // Reset result display
        const hashDisplay = document.getElementById('hash-display');
        if (hashDisplay) {
            hashDisplay.textContent = 'Start mining to reveal your hash!';
            hashDisplay.classList.remove('winner');
        }
    }

    // Force reveal (e.g., when mining stops)
    forceReveal() {
        if (!this.revealed) {
            this.revealAll();
        }
    }
}

export default ScratchCard;
