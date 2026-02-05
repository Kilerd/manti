import { useState, useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { client, type UserProfile } from "@/api";
import { toast } from "@/hooks/use-toast";
import { Check, X, AlertCircle } from "lucide-react";

interface PasswordRequirements {
  minLength: boolean;
  hasUpperCase: boolean;
  hasLowerCase: boolean;
  hasNumber: boolean;
  hasSpecialChar: boolean;
}

interface PasswordValidation {
  requirements: PasswordRequirements;
  strength: number;
  isValid: boolean;
}

function validatePassword(password: string): PasswordValidation {
  const requirements: PasswordRequirements = {
    minLength: password.length >= 8,
    hasUpperCase: /[A-Z]/.test(password),
    hasLowerCase: /[a-z]/.test(password),
    hasNumber: /\d/.test(password),
    hasSpecialChar: /[!@#$%^&*(),.?":{}|<>]/.test(password),
  };

  const strength = Object.values(requirements).filter(Boolean).length;
  const isValid = strength >= 4 && requirements.minLength;

  return { requirements, strength, isValid };
}

function PasswordStrengthIndicator({ password }: { password: string }) {
  if (!password) return null;

  const { requirements, strength } = validatePassword(password);

  const strengthLabel =
    ["Very Weak", "Weak", "Fair", "Good", "Strong"][strength] || "Very Weak";
  const strengthColor =
    [
      "text-red-500",
      "text-orange-500",
      "text-yellow-500",
      "text-blue-500",
      "text-green-500",
    ][strength] || "text-red-500";

  return (
    <div className="mt-2 space-y-2">
      <div className="flex items-center gap-2">
        <div className="flex-1 h-2 bg-gray-200 rounded-full overflow-hidden">
          <div
            className={`h-full transition-all ${
              strength === 1
                ? "bg-red-500"
                : strength === 2
                  ? "bg-orange-500"
                  : strength === 3
                    ? "bg-yellow-500"
                    : strength === 4
                      ? "bg-blue-500"
                      : strength === 5
                        ? "bg-green-500"
                        : ""
            }`}
            style={{ width: `${(strength / 5) * 100}%` }}
          />
        </div>
        <span className={`text-xs font-medium ${strengthColor}`}>
          {strengthLabel}
        </span>
      </div>
      <ul className="text-xs space-y-1">
        <li
          className={`flex items-center gap-1 ${requirements.minLength ? "text-green-600" : "text-gray-400"}`}
        >
          {requirements.minLength ? (
            <Check className="h-3 w-3" />
          ) : (
            <X className="h-3 w-3" />
          )}
          At least 8 characters
        </li>
        <li
          className={`flex items-center gap-1 ${requirements.hasUpperCase ? "text-green-600" : "text-gray-400"}`}
        >
          {requirements.hasUpperCase ? (
            <Check className="h-3 w-3" />
          ) : (
            <X className="h-3 w-3" />
          )}
          One uppercase letter
        </li>
        <li
          className={`flex items-center gap-1 ${requirements.hasLowerCase ? "text-green-600" : "text-gray-400"}`}
        >
          {requirements.hasLowerCase ? (
            <Check className="h-3 w-3" />
          ) : (
            <X className="h-3 w-3" />
          )}
          One lowercase letter
        </li>
        <li
          className={`flex items-center gap-1 ${requirements.hasNumber ? "text-green-600" : "text-gray-400"}`}
        >
          {requirements.hasNumber ? (
            <Check className="h-3 w-3" />
          ) : (
            <X className="h-3 w-3" />
          )}
          One number
        </li>
        <li
          className={`flex items-center gap-1 ${requirements.hasSpecialChar ? "text-green-600" : "text-gray-400"}`}
        >
          {requirements.hasSpecialChar ? (
            <Check className="h-3 w-3" />
          ) : (
            <X className="h-3 w-3" />
          )}
          One special character
        </li>
      </ul>
    </div>
  );
}

interface FormErrors {
  name?: string;
  currentPassword?: string;
  newPassword?: string;
  confirmPassword?: string;
}

export default function Profile() {
  const [profile, setProfile] = useState<UserProfile>({
    id: "",
    name: "",
    email: "",
    createdAt: "",
  });
  const [editing, setEditing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [passwordErrors, setPasswordErrors] = useState<FormErrors>({});
  const [formData, setFormData] = useState({
    name: "",
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
  });

  useEffect(() => {
    fetchProfile();
  }, []);

  const fetchProfile = async () => {
    try {
      const { data, error } = await client.GET("/user/profile");

      if (error || !data) {
        throw new Error("Failed to fetch profile");
      }

      setProfile(data);
      setFormData((prev) => ({ ...prev, name: data.name }));
    } catch {
      toast({
        title: "Error",
        description: "Failed to fetch profile. Please try again.",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const validateForm = (): boolean => {
    const errors: FormErrors = {};

    if (!formData.name.trim()) {
      errors.name = "Name is required";
    }

    if (
      formData.newPassword ||
      formData.currentPassword ||
      formData.confirmPassword
    ) {
      if (!formData.currentPassword) {
        errors.currentPassword = "Current password is required";
      }

      if (!formData.newPassword) {
        errors.newPassword = "New password is required";
      } else {
        const { isValid } = validatePassword(formData.newPassword);
        if (!isValid) {
          errors.newPassword = "Password does not meet strength requirements";
        }
      }

      if (formData.newPassword !== formData.confirmPassword) {
        errors.confirmPassword = "Passwords do not match";
      }

      if (formData.newPassword === formData.currentPassword) {
        errors.newPassword =
          "New password must be different from current password";
      }
    }

    setPasswordErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSave = async () => {
    if (!validateForm()) {
      toast({
        title: "Validation Error",
        description: "Please fix the errors before saving.",
        variant: "destructive",
      });
      return;
    }

    setSaving(true);
    try {
      const updateData: {
        name?: string;
        currentPassword?: string;
        newPassword?: string;
      } = { name: formData.name };

      if (formData.newPassword) {
        updateData.currentPassword = formData.currentPassword;
        updateData.newPassword = formData.newPassword;
      }

      const { error } = await client.PUT("/user/profile", {
        body: updateData,
      });

      if (error) {
        throw new Error(
          (error as { message?: string })?.message ||
            "Failed to update profile"
        );
      }

      setProfile({ ...profile, name: formData.name });
      setEditing(false);
      setFormData({
        ...formData,
        currentPassword: "",
        newPassword: "",
        confirmPassword: "",
      });
      setPasswordErrors({});

      toast({
        title: "Profile Updated",
        description: "Your profile has been successfully updated.",
      });
    } catch (err) {
      const message =
        err instanceof Error
          ? err.message
          : "Failed to update profile. Please try again.";
      toast({
        title: "Update Failed",
        description: message,
        variant: "destructive",
      });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h1 className="text-3xl font-bold">Profile</h1>
        <p className="text-muted-foreground">Manage your account settings</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Account Information</CardTitle>
          <CardDescription>
            Your personal details and account settings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input id="email" value={profile.email} disabled />
          </div>

          <div className="space-y-2">
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={editing ? formData.name : profile.name}
              onChange={(e) => {
                setFormData({ ...formData, name: e.target.value });
                setPasswordErrors({ ...passwordErrors, name: undefined });
              }}
              disabled={!editing}
              className={passwordErrors.name ? "border-destructive" : ""}
            />
            {passwordErrors.name && (
              <p className="text-xs text-destructive flex items-center gap-1">
                <AlertCircle className="h-3 w-3" />
                {passwordErrors.name}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label>Member Since</Label>
            <Input
              value={new Date(profile.createdAt).toLocaleDateString()}
              disabled
            />
          </div>

          {editing && (
            <>
              <div className="border-t pt-4">
                <h3 className="text-sm font-medium mb-4">
                  Change Password (Optional)
                </h3>
                <div className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="currentPassword">Current Password</Label>
                    <Input
                      id="currentPassword"
                      type="password"
                      value={formData.currentPassword}
                      onChange={(e) => {
                        setFormData({
                          ...formData,
                          currentPassword: e.target.value,
                        });
                        setPasswordErrors({
                          ...passwordErrors,
                          currentPassword: undefined,
                        });
                      }}
                      className={
                        passwordErrors.currentPassword
                          ? "border-destructive"
                          : ""
                      }
                    />
                    {passwordErrors.currentPassword && (
                      <p className="text-xs text-destructive flex items-center gap-1">
                        <AlertCircle className="h-3 w-3" />
                        {passwordErrors.currentPassword}
                      </p>
                    )}
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="newPassword">New Password</Label>
                    <Input
                      id="newPassword"
                      type="password"
                      value={formData.newPassword}
                      onChange={(e) => {
                        setFormData({
                          ...formData,
                          newPassword: e.target.value,
                        });
                        setPasswordErrors({
                          ...passwordErrors,
                          newPassword: undefined,
                        });
                      }}
                      className={
                        passwordErrors.newPassword ? "border-destructive" : ""
                      }
                    />
                    {passwordErrors.newPassword && (
                      <p className="text-xs text-destructive flex items-center gap-1">
                        <AlertCircle className="h-3 w-3" />
                        {passwordErrors.newPassword}
                      </p>
                    )}
                    <PasswordStrengthIndicator password={formData.newPassword} />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="confirmPassword">Confirm New Password</Label>
                    <Input
                      id="confirmPassword"
                      type="password"
                      value={formData.confirmPassword}
                      onChange={(e) => {
                        setFormData({
                          ...formData,
                          confirmPassword: e.target.value,
                        });
                        setPasswordErrors({
                          ...passwordErrors,
                          confirmPassword: undefined,
                        });
                      }}
                      className={
                        passwordErrors.confirmPassword
                          ? "border-destructive"
                          : ""
                      }
                    />
                    {passwordErrors.confirmPassword && (
                      <p className="text-xs text-destructive flex items-center gap-1">
                        <AlertCircle className="h-3 w-3" />
                        {passwordErrors.confirmPassword}
                      </p>
                    )}
                  </div>
                </div>
              </div>
            </>
          )}

          <div className="flex justify-end space-x-2 pt-4">
            {!editing ? (
              <Button onClick={() => setEditing(true)}>Edit Profile</Button>
            ) : (
              <>
                <Button
                  variant="outline"
                  onClick={() => {
                    setEditing(false);
                    setPasswordErrors({});
                    setFormData({
                      name: profile.name,
                      currentPassword: "",
                      newPassword: "",
                      confirmPassword: "",
                    });
                  }}
                >
                  Cancel
                </Button>
                <Button onClick={handleSave} disabled={saving}>
                  {saving ? "Saving..." : "Save Changes"}
                </Button>
              </>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
