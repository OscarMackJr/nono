// scripts/azure/clean-vm/main.bicep
//
// Phase 103 (CHOST-01) - Azure clean-host VM IaC.
//
// Self-contained Bicep module: declares its own VNet/Subnet/NSG/PublicIP/NIC/VM. Does NOT
// reference any pre-existing RG_Nono resource by name or ID (RG_Nono currently holds only the
// live ArtifactNono code-signing account, with no reusable networking) - the target resource
// group is chosen entirely at deploy time via `az deployment group create -g <rg>`.
//
// Purpose: a reproducible, ephemeral Gen2 + Trusted-Launch Windows 11 test fixture used by
// Phase 106 clean-host UAT to prove a real trusted-signed nono release installs and the broker
// spawns with no manual cert-trust step and no VC++ redist confound. This module is
// author + `az bicep build`-validated ONLY this phase - no live `az deployment` is run here.
//
// Security posture (see 103-01-PLAN.md <threat_model> T-103-01/04):
//   - `vmImageSku` and `operatorIpCidr` are REQUIRED parameters with NO default value. A stale
//     hardcoded SKU or a wildcard NSG rule is structurally impossible to author here by mistake -
//     the only way to supply a value is deploy.ps1's live `az vm image list-skus`/`curl`
//     resolution immediately before deploy.
//   - The NSG authors exactly one inbound allow rule (RDP, scoped to `operatorIpCidr`, never a
//     wildcard any-address CIDR) plus an explicit deny-all-other-inbound rule as defense in depth
//     beyond Azure's own default deny.

@description('Azure region for all resources. Defaults to eastus (the live-verified region used for the win11 SKU query in 103-RESEARCH.md).')
param location string = 'eastus'

@description('Windows 11 SKU, resolved LIVE via `az vm image list-skus` in deploy.ps1 - NEVER hardcode a default here. The live SKU catalog under MicrosoftWindowsDesktop/windows-11 rotates (e.g. win11-23h2-* already superseded by 24h2/25h2) so a default would silently go stale.')
param vmImageSku string

@description('Operator public IP in CIDR form (e.g. 1.2.3.4/32), fetched via `curl` in deploy.ps1 - NEVER `az rest`, NEVER a hardcoded/literal value. This is the sole source scope for the RDP inbound rule; the NSG never authors a wildcard any-address CIDR.')
param operatorIpCidr string

@description('VM computer name / resource name.')
param vmName string = 'nono-clean-host'

@description('VM size.')
param vmSize string = 'Standard_D4s_v5'

@description('Local administrator username for the clean-host VM.')
param adminUsername string

@description('Local administrator password for the clean-host VM.')
@secure()
param adminPassword string

var vnetName = 'nono-clean-vnet'
var vnetAddressPrefix = '10.20.0.0/24'
var subnetName = 'default'
var subnetAddressPrefix = '10.20.0.0/25'
var nsgName = 'nono-clean-nsg'
var pipName = 'nono-clean-pip'
var nicName = 'nono-clean-nic'

// (2) NSG - exactly two rules: allow RDP from the operator's own IP only, deny everything else
// inbound. Declared before `vnet` in resource-graph terms via the vnet subnet's
// `networkSecurityGroup.id` reference below (textual order in this file matches the plan's
// 1-vnet/2-nsg narrative order; Bicep resolves the dependency graph regardless of declaration
// order, so this forward reference from `vnet` to `nsg` is valid).
resource nsg 'Microsoft.Network/networkSecurityGroups@2023-11-01' = {
  name: nsgName
  location: location
  properties: {
    securityRules: [
      {
        name: 'AllowRdpFromOperator'
        properties: {
          priority: 100
          direction: 'Inbound'
          access: 'Allow'
          protocol: 'Tcp'
          sourcePortRange: '*'
          destinationPortRange: '3389'
          sourceAddressPrefix: operatorIpCidr
          destinationAddressPrefix: '*'
        }
      }
      {
        name: 'DenyAllOtherInbound'
        properties: {
          priority: 4096
          direction: 'Inbound'
          access: 'Deny'
          protocol: '*'
          sourcePortRange: '*'
          destinationPortRange: '*'
          sourceAddressPrefix: '*'
          destinationAddressPrefix: '*'
        }
      }
    ]
  }
}

// (1) VNet + subnet, NSG applied at the subnet level (ASVS V4 least-privilege boundary placement).
resource vnet 'Microsoft.Network/virtualNetworks@2023-11-01' = {
  name: vnetName
  location: location
  properties: {
    addressSpace: {
      addressPrefixes: [
        vnetAddressPrefix
      ]
    }
    subnets: [
      {
        name: subnetName
        properties: {
          addressPrefix: subnetAddressPrefix
          networkSecurityGroup: {
            id: nsg.id
          }
        }
      }
    ]
  }
}

// (3) Public IP - Standard SKU requires Static allocation.
resource pip 'Microsoft.Network/publicIPAddresses@2023-11-01' = {
  name: pipName
  location: location
  sku: {
    name: 'Standard'
  }
  properties: {
    publicIPAllocationMethod: 'Static'
  }
}

// (4) NIC - single ipConfiguration binding the vnet's `default` subnet and the `pip` resource.
resource nic 'Microsoft.Network/networkInterfaces@2023-11-01' = {
  name: nicName
  location: location
  properties: {
    ipConfigurations: [
      {
        name: 'ipconfig1'
        properties: {
          subnet: {
            id: vnet.properties.subnets[0].id
          }
          publicIPAddress: {
            id: pip.id
          }
          privateIPAllocationMethod: 'Dynamic'
        }
      }
    ]
  }
}

// (5) VM - Gen2 + Trusted-Launch (secureBoot + vTPM), Windows 11 image with a live-resolved SKU.
resource vm 'Microsoft.Compute/virtualMachines@2024-07-01' = {
  name: vmName
  location: location
  properties: {
    hardwareProfile: {
      vmSize: vmSize
    }
    storageProfile: {
      imageReference: {
        publisher: 'MicrosoftWindowsDesktop'
        offer: 'windows-11'
        sku: vmImageSku
        version: 'latest'
      }
      osDisk: {
        createOption: 'FromImage'
        managedDisk: {
          storageAccountType: 'Premium_LRS'
        }
      }
    }
    osProfile: {
      computerName: vmName
      adminUsername: adminUsername
      adminPassword: adminPassword
    }
    securityProfile: {
      securityType: 'TrustedLaunch'
      uefiSettings: {
        secureBootEnabled: true
        vTpmEnabled: true
      }
    }
    networkProfile: {
      networkInterfaces: [
        {
          id: nic.id
        }
      ]
    }
  }
}

@description('Resource ID of the deployed VM.')
output vmId string = vm.id

@description('Fully qualified domain name-free public IP resource ID (the assigned address itself is only known post-deploy; fetch via `az network public-ip show` after `deploy.ps1` runs).')
output publicIpId string = pip.id
