import {ethers} from "hardhat";
import "@nomiclabs/hardhat-etherscan";
import {expect} from "chai";
import fs from 'fs';

describe("Groth16", function () {
    it("Should return true when proof is correct", async function () {
        const verifierFactory = await ethers.getContractFactory("Verifier");
        const verifier = await verifierFactory.deploy();
        await verifier.deployed();

        const publicJson = fs.readFileSync("../public.json").toString();
        const publicInputs: any[] = JSON.parse(publicJson);
        const proofJson = fs.readFileSync("../proof.json").toString();
        const proof = JSON.parse(proofJson);

        expect(await verifier.verifyProof(
            "0x" + proof.proof,
            publicInputs
        )).to.equal(true);
    });
});
